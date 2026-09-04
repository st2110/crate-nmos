//! The vendored AMWA schemas are the contract this project is checked against,
//! so the tree itself is verified: every `$ref` must resolve to a file that is
//! actually here, and every resource this project consumes must have at least
//! one example to round-trip.
//!
//! A dangling reference would otherwise surface much later, as a confusing
//! validator failure, rather than as a re-vendor that missed a file.

// This file is test code in its entirety, which is the one place AGENTS.md
// allows a panic: an assertion that fails loudly beats an error that is
// propagated into a passing test.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The `schemas/` directory at the repository root.
fn schemas_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas")
}

/// Every `*.json` file directly inside `dir`, sorted.
fn json_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
}

/// Collect the value of every `$ref` appearing anywhere in `value`.
fn collect_refs(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == "$ref"
                    && let Value::String(target) = child
                {
                    out.insert(target.clone());
                }
                collect_refs(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_refs(item, out);
            }
        }
        _ => {}
    }
}

/// Walk a JSON pointer such as `/definitions/constraint`, returning whether it
/// resolves inside `document`.
fn pointer_resolves(document: &Value, pointer: &str) -> bool {
    document.pointer(pointer).is_some()
}

/// Report every `$ref` in `dir` that does not resolve to a vendored file, and
/// every fragment that does not resolve inside the file it names.
fn dangling_refs(dir: &Path) -> Vec<String> {
    let mut problems = Vec::new();

    for path in json_files(dir) {
        let text = std::fs::read_to_string(&path).expect("schema is readable");
        let document: Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", path.display()));

        let mut refs = BTreeSet::new();
        collect_refs(&document, &mut refs);

        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        for reference in refs {
            let (file, fragment) = match reference.split_once('#') {
                Some((file, fragment)) => (file, Some(fragment)),
                None => (reference.as_str(), None),
            };

            // A bare fragment points inside the document that carries it.
            let target_path = if file.is_empty() {
                path.clone()
            } else {
                dir.join(file)
            };

            if !target_path.is_file() {
                problems.push(format!(
                    "{name}: $ref {reference} -> missing {}",
                    target_path.display()
                ));
                continue;
            }

            if let Some(fragment) = fragment {
                if fragment.is_empty() {
                    continue;
                }
                let target_text =
                    std::fs::read_to_string(&target_path).expect("target is readable");
                let target: Value =
                    serde_json::from_str(&target_text).expect("target is valid JSON");
                if !pointer_resolves(&target, fragment) {
                    problems.push(format!(
                        "{name}: $ref {reference} -> fragment does not resolve"
                    ));
                }
            }
        }
    }

    problems
}

#[test]
fn every_is_04_reference_resolves_to_a_vendored_file() {
    let dir = schemas_root().join("is-04/v1.3");
    let problems = dangling_refs(&dir);
    assert!(
        problems.is_empty(),
        "dangling references:\n{}",
        problems.join("\n")
    );
}

#[test]
fn every_is_05_reference_resolves_to_a_vendored_file() {
    let dir = schemas_root().join("is-05/v1.1");
    let problems = dangling_refs(&dir);
    assert!(
        problems.is_empty(),
        "dangling references:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_reference_check_catches_a_dangling_reference() {
    // The check is only worth having if it fails on a broken tree, so prove it
    // does rather than trusting that two green tests mean anything.
    let dir = tempdir();
    std::fs::write(
        dir.join("a.json"),
        r#"{"allOf": [{"$ref": "not_vendored.json"}]}"#,
    )
    .expect("fixture is writable");

    let problems = dangling_refs(&dir);
    assert_eq!(
        problems.len(),
        1,
        "expected exactly one problem, got {problems:?}"
    );
    assert!(problems[0].contains("not_vendored.json"));
}

#[test]
fn the_reference_check_catches_an_unresolvable_fragment() {
    let dir = tempdir();
    std::fs::write(
        dir.join("a.json"),
        r#"{"$ref": "b.json#/definitions/absent"}"#,
    )
    .expect("fixture is writable");
    std::fs::write(dir.join("b.json"), r#"{"definitions": {"present": {}}}"#)
        .expect("fixture is writable");

    let problems = dangling_refs(&dir);
    assert_eq!(
        problems.len(),
        1,
        "expected exactly one problem, got {problems:?}"
    );
    assert!(problems[0].contains("fragment does not resolve"));
}

/// Resources this change consumes, and the example filename fragment that must
/// exist for each. A resource with no example is a resource the round-trip
/// tests silently skip.
const REQUIRED_IS_04_EXAMPLES: &[&str] = &[
    "nodeapi-self",
    "nodeapi-devices",
    "nodeapi-senders",
    "nodeapi-receivers",
    "nodeapi-flows",
    "nodeapi-sources",
];

const REQUIRED_IS_05_EXAMPLES: &[&str] = &["sender-active", "receiver-active"];

#[test]
fn every_consumed_resource_has_at_least_one_example() {
    let is_04 = json_files(&schemas_root().join("examples/is-04"));
    assert!(!is_04.is_empty(), "no IS-04 examples vendored");
    for required in REQUIRED_IS_04_EXAMPLES {
        assert!(
            is_04.iter().any(|p| p.to_string_lossy().contains(required)),
            "no vendored IS-04 example for {required}"
        );
    }

    let is_05 = json_files(&schemas_root().join("examples/is-05"));
    assert!(!is_05.is_empty(), "no IS-05 examples vendored");
    for required in REQUIRED_IS_05_EXAMPLES {
        assert!(
            is_05.iter().any(|p| p.to_string_lossy().contains(required)),
            "no vendored IS-05 example for {required}"
        );
    }
}

#[test]
fn every_vendored_example_is_valid_json() {
    for dir in ["examples/is-04", "examples/is-05"] {
        for path in json_files(&schemas_root().join(dir)) {
            let text = std::fs::read_to_string(&path).expect("example is readable");
            serde_json::from_str::<Value>(&text)
                .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", path.display()));
        }
    }
}

/// A scratch directory under the target directory, unique per test.
fn tempdir() -> PathBuf {
    let unique = format!(
        "nmos-schema-test-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    );
    let dir = std::env::temp_dir().join(unique);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory is creatable");
    dir
}
