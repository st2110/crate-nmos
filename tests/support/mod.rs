//! Shared test harness: builds validators from the vendored AMWA schemas.
//!
//! The schemas are JSON Schema draft-04 and refer to each other by bare
//! filename (`{"$ref": "resource_core.json"}`). They are given a synthetic base
//! URI and a retriever that maps that URI back onto `schemas/`, so validation
//! resolves the whole `allOf` chain without the crate's file or HTTP retrieval
//! features and without touching the network — which is what
//! `DESIGN.md`, "How correctness is held" requires.

#![allow(dead_code)]

pub mod fixture;

use std::path::{Path, PathBuf};

use jsonschema::{Draft, Retrieve, Uri, Validator};
use serde_json::Value;

/// A host that is guaranteed never to resolve (RFC 2606), so a mistake in the
/// retriever cannot silently become a network fetch.
const BASE: &str = "https://schemas.nmos.invalid/";

/// Which vendored specification a schema belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spec {
    Is04,
    Is05,
}

impl Spec {
    /// The directory under `schemas/` holding this specification.
    fn dir(self) -> &'static str {
        match self {
            Spec::Is04 => "is-04/v1.3",
            Spec::Is05 => "is-05/v1.1",
        }
    }

    /// The directory under `schemas/examples/` holding its examples.
    fn examples_dir(self) -> &'static str {
        match self {
            Spec::Is04 => "examples/is-04",
            Spec::Is05 => "examples/is-05",
        }
    }
}

/// The `schemas/` directory at the repository root.
pub fn schemas_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas")
}

/// Serves the vendored schemas to the validator, and nothing else.
struct Vendored {
    root: PathBuf,
}

impl Retrieve for Vendored {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let path = uri.path().as_str().trim_start_matches('/');
        let file = self.root.join(path);
        if !file.starts_with(&self.root) {
            return Err(format!("{uri} escapes the vendored schema tree").into());
        }
        let text = std::fs::read_to_string(&file)
            .map_err(|e| format!("{uri} -> {}: {e}", file.display()))?;
        Ok(serde_json::from_str(&text)?)
    }
}

/// The parsed contents of a vendored schema.
pub fn schema(spec: Spec, file: &str) -> Value {
    let path = schemas_root().join(spec.dir()).join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("vendored schema {} is missing: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("vendored schema {} is not valid JSON: {e}", path.display()))
}

/// A validator for one vendored schema, with its siblings resolvable.
///
/// # Panics
///
/// Panics if the schema is absent, is not draft-04, or refers to something the
/// vendored tree does not contain — each of which is a broken checkout rather
/// than a test failure worth reporting politely.
pub fn validator(spec: Spec, file: &str) -> Validator {
    let contents = schema(spec, file);

    assert_eq!(
        Draft::default().detect(&contents),
        Draft::Draft4,
        "{file} is not draft-04; the vendored contract has changed shape"
    );

    let base = format!("{BASE}{}/{file}", spec.dir());
    jsonschema::draft4::options()
        .with_base_uri(base)
        .with_retriever(Vendored {
            root: schemas_root(),
        })
        .build(&contents)
        .unwrap_or_else(|e| panic!("cannot build a validator for {file}: {e}"))
}

/// Assert that `instance` satisfies a vendored schema, reporting every failure
/// rather than only the first.
pub fn assert_valid(spec: Spec, file: &str, instance: &Value) {
    let validator = validator(spec, file);
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("  {}: {e}", e.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "instance does not satisfy {file}:\n{}",
        errors.join("\n")
    );
}

/// Read a vendored example document.
pub fn example(spec: Spec, file: &str) -> Value {
    let path = schemas_root().join(spec.examples_dir()).join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("vendored example {} is missing: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("vendored example {} is not valid JSON: {e}", path.display()))
}
