//! The harness in `support` is what every later round-trip test leans on, so it
//! is tested in its own right: a harness that cannot fail is a harness that
//! proves nothing.

// This file is test code in its entirety.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use serde_json::json;
use support::{Spec, assert_valid, example, schema, validator};

#[test]
fn the_vendored_schemas_are_draft_04() {
    // `validator` asserts this itself; naming it here says out loud that
    // draft-04 is the constraint the whole schema strategy turns on.
    for file in ["node.json", "device.json", "sender.json", "receiver.json"] {
        assert_eq!(
            schema(Spec::Is04, file)["$schema"],
            json!("http://json-schema.org/draft-04/schema#")
        );
    }
}

#[test]
fn a_schema_with_sibling_references_builds() {
    // `node.json` is `allOf: [resource_core.json, ...]`, so building it at all
    // proves the retriever resolves references across vendored files.
    let _ = validator(Spec::Is04, "node.json");
    let _ = validator(Spec::Is04, "sender.json");
    let _ = validator(Spec::Is05, "sender-response-schema.json");
}

#[test]
fn the_published_examples_satisfy_their_own_schemas() {
    assert_valid(
        Spec::Is04,
        "node.json",
        &example(Spec::Is04, "nodeapi-self-get-200.json"),
    );
    assert_valid(
        Spec::Is04,
        "devices.json",
        &example(Spec::Is04, "nodeapi-devices-get-200.json"),
    );
    assert_valid(
        Spec::Is04,
        "senders.json",
        &example(Spec::Is04, "nodeapi-senders-get-200.json"),
    );
    assert_valid(
        Spec::Is04,
        "receivers.json",
        &example(Spec::Is04, "nodeapi-receivers-get-200.json"),
    );
    assert_valid(
        Spec::Is04,
        "flows.json",
        &example(Spec::Is04, "nodeapi-flows-get-200.json"),
    );
    assert_valid(
        Spec::Is04,
        "sources.json",
        &example(Spec::Is04, "nodeapi-sources-get-200.json"),
    );
}

#[test]
fn the_harness_rejects_a_document_that_is_wrong() {
    // A Node record missing every required field. If this passed, every
    // round-trip test built on the harness would be worthless.
    let validator = validator(Spec::Is04, "node.json");
    assert!(!validator.is_valid(&json!({})));
    assert!(validator.validate(&json!({})).is_err());
}

#[test]
fn the_harness_rejects_a_document_that_is_subtly_wrong() {
    // Present but of the wrong type: the failure mode a hand-written Rust type
    // is most likely to introduce.
    let mut node = example(Spec::Is04, "nodeapi-self-get-200.json");
    node["version"] = json!(1_234);

    let validator = validator(Spec::Is04, "node.json");
    assert!(
        !validator.is_valid(&node),
        "a numeric `version` must not satisfy the Node schema"
    );
}

#[test]
fn the_harness_reports_every_failure_not_only_the_first() {
    let validator = validator(Spec::Is04, "node.json");
    let count = validator.iter_errors(&json!({})).count();
    assert!(
        count > 1,
        "expected several failures for an empty Node, got {count}"
    );
}
