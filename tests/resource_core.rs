//! Round-trip tests for the fields every NMOS resource shares.
//!
//! The contract is checked in both directions, which is the whole point of
//! `DESIGN.md`, "How correctness is held": every published
//! example must parse into our types, and everything our types serialize must
//! validate against the schema that governs it.

// This file is test code in its entirety.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use nmos::{ResourceCore, ResourceId, Version};
use serde_json::json;
use support::{Spec, assert_valid, example};

/// Every published Node API example, paired with the schema governing one of
/// its elements. The collection endpoints return arrays of resources.
const COLLECTIONS: &[(&str, &str)] = &[
    ("nodeapi-devices-get-200.json", "device.json"),
    ("nodeapi-senders-get-200.json", "sender.json"),
    ("nodeapi-receivers-get-200.json", "receiver.json"),
    ("nodeapi-flows-get-200.json", "flow.json"),
    ("nodeapi-sources-get-200.json", "source.json"),
];

#[test]
fn the_core_fields_of_every_published_example_parse() {
    let node = example(Spec::Is04, "nodeapi-self-get-200.json");
    let core: ResourceCore = serde_json::from_value(node).expect("the Node example parses");
    assert!(!core.id.as_str().is_empty());

    for (file, _) in COLLECTIONS {
        let collection = example(Spec::Is04, file);
        let items = collection.as_array().expect("a collection is an array");
        assert!(!items.is_empty(), "{file} has no elements to exercise");
        for item in items {
            let _: ResourceCore =
                serde_json::from_value(item.clone()).unwrap_or_else(|e| panic!("{file}: {e}"));
        }
    }
}

#[test]
fn a_serialized_core_satisfies_resource_core_json() {
    let node = example(Spec::Is04, "nodeapi-self-get-200.json");
    let core: ResourceCore = serde_json::from_value(node).expect("the Node example parses");
    let round_tripped = serde_json::to_value(&core).expect("a core serializes");
    assert_valid(Spec::Is04, "resource_core.json", &round_tripped);
}

#[test]
fn every_published_example_round_trips_through_the_core() {
    for (file, _) in COLLECTIONS {
        let collection = example(Spec::Is04, file);
        for item in collection.as_array().expect("a collection is an array") {
            let core: ResourceCore = serde_json::from_value(item.clone()).expect("parses");
            let round_tripped = serde_json::to_value(&core).expect("serializes");
            assert_valid(Spec::Is04, "resource_core.json", &round_tripped);
        }
    }
}

#[test]
fn a_resource_missing_its_label_still_parses() {
    // The specification requires `label`, but equipment omits it, and the specs
    // require such a Node to be listed under its hostname rather than dropped.
    let core: ResourceCore = serde_json::from_value(json!({
        "id": "3b8be755-08ff-452b-b217-c9151eb21193",
        "version": "1441700172:318426300",
        "description": "",
        "tags": {}
    }))
    .expect("a resource without a label parses");
    assert_eq!(core.label, "");
}

#[test]
fn tags_survive_a_round_trip() {
    let core: ResourceCore = serde_json::from_value(json!({
        "id": "3b8be755-08ff-452b-b217-c9151eb21193",
        "version": "1441700172:318426300",
        "label": "SDI 1",
        "description": "",
        "tags": {"urn:x-nmos:tag:grouphint/v1.0": ["SDI 1:0"], "empty": []}
    }))
    .expect("parses");

    let round_tripped = serde_json::to_value(&core).expect("serializes");
    assert_eq!(
        round_tripped["tags"]["urn:x-nmos:tag:grouphint/v1.0"],
        json!(["SDI 1:0"])
    );
    assert_eq!(round_tripped["tags"]["empty"], json!([]));
    assert_valid(Spec::Is04, "resource_core.json", &round_tripped);
}

// --- identifiers ------------------------------------------------------------

#[test]
fn a_well_formed_identifier_parses_and_prints_back_unchanged() {
    let text = "3b8be755-08ff-452b-b217-c9151eb21193";
    let id: ResourceId = text.parse().expect("a well-formed identifier parses");
    assert_eq!(id.as_str(), text);
    assert_eq!(id.to_string(), text);
}

#[test]
fn a_malformed_identifier_is_rejected() {
    let rejected = [
        "",
        "not-a-uuid",
        "3b8be755-08ff-452b-b217-c9151eb2119", // one character short
        "3b8be755-08ff-452b-b217-c9151eb211933", // one character long
        "3B8BE755-08FF-452B-B217-C9151EB21193", // upper case: the schema is lower case
        "3b8be755:08ff:452b:b217:c9151eb21193", // wrong separators
        "3b8be755-08ff-052b-b217-c9151eb21193", // version nibble 0, outside 1-5
        "3b8be755-08ff-452b-c217-c9151eb21193", // variant nibble c, outside 8-b
        "00000000-0000-0000-0000-000000000000", // the nil UUID is not NMOS-valid
        "3b8be755-08ff-452b-b217-c9151eb211g3", // non-hex character
    ];
    for text in rejected {
        assert!(
            text.parse::<ResourceId>().is_err(),
            "`{text}` must not parse as an identifier"
        );
    }
}

#[test]
fn an_identifier_rejected_at_parse_time_is_rejected_when_deserialized() {
    assert!(serde_json::from_value::<ResourceId>(json!("not-a-uuid")).is_err());
    assert!(serde_json::from_value::<ResourceId>(json!(42)).is_err());
}

// --- versions ---------------------------------------------------------------

#[test]
fn a_version_parses_into_its_two_parts() {
    let version: Version = "1441700172:318426300".parse().expect("parses");
    assert_eq!(version.seconds(), 1_441_700_172);
    assert_eq!(version.nanoseconds(), 318_426_300);
    assert_eq!(version.to_string(), "1441700172:318426300");
}

#[test]
fn versions_order_by_time() {
    let earlier: Version = "1441700172:318426300".parse().expect("parses");
    let later: Version = "1441700172:318426301".parse().expect("parses");
    let much_later: Version = "1441700173:0".parse().expect("parses");

    assert!(earlier < later);
    assert!(later < much_later);
    assert_eq!(earlier, earlier);
}

#[test]
fn a_malformed_version_is_rejected() {
    let rejected = [
        "",
        "1441700172",
        "1441700172:",
        ":318426300",
        "1441700172:318426300:1",
        "-1:0",
        "1441700172.318426300",
        "abc:def",
        " 1441700172:318426300",
    ];
    for text in rejected {
        assert!(
            text.parse::<Version>().is_err(),
            "`{text}` must not parse as a version"
        );
    }
}

#[test]
fn a_version_too_large_to_hold_is_an_error_not_a_panic() {
    // The schema's pattern allows arbitrarily many digits. Refusing them is
    // correct; aborting on them is not.
    let huge = format!("{}:0", "9".repeat(40));
    assert!(huge.parse::<Version>().is_err());
}
