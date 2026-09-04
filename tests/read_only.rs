//! This change reads. It does not write.
//!
//! That is a contract, not an intention, so it is checked: the crate must
//! expose no way to stage a connection, activate one, or otherwise change a
//! device. A controller that can accidentally reconfigure equipment which is on
//! air is not one anybody should run, and the cheapest moment to catch such a
//! method is the commit that adds it.

// This file is test code in its entirety.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

/// Words that would mean this crate had gained the ability to write.
const WRITING: &[(&str, &str)] = &[
    ("staged", "staging a connection is a write"),
    ("activate", "activation is a write"),
    ("activation", "activation is a write"),
    ("patch", "PATCH is how IS-05 writes"),
    ("post", "POST is how IS-04 registers"),
    ("put", "PUT is a write"),
    ("delete", "DELETE is a write"),
    ("bulk", "the bulk endpoint exists only to write"),
];

fn crate_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn source_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(&crate_src(), &mut files);
    files.sort();
    files
}

/// The identifier declared by a line introducing a public item, if any.
fn public_identifier(line: &str) -> Option<&str> {
    let rest = line.trim().strip_prefix("pub ")?;
    if rest.starts_with('(') {
        return None;
    }
    // Strip modifiers before the item keyword, so `pub async fn patch_staged`
    // is seen as `patch_staged` and not as `async`.
    let mut rest = rest;
    loop {
        let stripped = ["async ", "unsafe ", "extern ", "default "]
            .iter()
            .find_map(|kw| rest.strip_prefix(kw));
        match stripped {
            Some(next) => rest = next.trim_start(),
            None => break,
        }
    }
    let rest = [
        "fn ", "struct ", "enum ", "trait ", "const ", "static ", "type ", "mod ", "union ",
    ]
    .iter()
    .find_map(|kw| rest.strip_prefix(kw))
    .unwrap_or(rest);
    rest.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .find(|s| !s.is_empty())
}

fn words(identifier: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in identifier.chars() {
        if ch == '_' {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
        } else if ch.is_uppercase() && !current.is_empty() {
            out.push(std::mem::take(&mut current));
            current.push(ch.to_ascii_lowercase());
        } else {
            current.push(ch.to_ascii_lowercase());
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

#[test]
fn the_public_surface_offers_no_way_to_write_to_a_device() {
    let mut problems = Vec::new();
    for path in source_files() {
        let text = std::fs::read_to_string(&path).expect("source is readable");
        for (number, line) in text.lines().enumerate() {
            let Some(identifier) = public_identifier(line) else {
                continue;
            };
            let parts = words(identifier);
            for (word, why) in WRITING {
                if parts.iter().any(|p| p == word) {
                    problems.push(format!(
                        "{}:{}: `{identifier}` — {why}",
                        path.display(),
                        number + 1
                    ));
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "this crate is read-only by contract:\n{}",
        problems.join("\n")
    );
}

#[test]
fn no_request_in_this_crate_uses_a_writing_method() {
    // The surface check would miss a write made from inside a
    // read-shaped function, so the calls themselves are checked too.
    let mut problems = Vec::new();
    for path in source_files() {
        let text = std::fs::read_to_string(&path).expect("source is readable");
        for (number, line) in text.lines().enumerate() {
            for method in [".post(", ".put(", ".patch(", ".delete(", ".head("] {
                if line.contains(method) {
                    problems.push(format!("{}:{}: {method}", path.display(), number + 1));
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "an http call in this crate writes:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_read_only_check_would_catch_a_write() {
    assert_eq!(
        public_identifier("pub fn patch_staged() {"),
        Some("patch_staged")
    );
    assert_eq!(
        public_identifier("pub async fn patch_staged() {"),
        Some("patch_staged")
    );
    assert_eq!(
        public_identifier("pub unsafe fn activate() {"),
        Some("activate")
    );
    assert!(words("patch_staged").contains(&"patch".to_owned()));
    assert!(words("ActivateImmediate").contains(&"activate".to_owned()));
    assert_eq!(public_identifier("    fn patch_staged() {"), None);
}
