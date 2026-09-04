//! Equipment sends fields we do not know about — vendor extensions, and fields
//! from IS-04 versions newer than the ones this project models. Parsing must
//! survive them, and their presence must change nothing else.

// This file is test code in its entirety.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use nmos::{Device, Flow, Node, Receiver, Sender, Source};
use serde_json::{Value, json};
use support::{Spec, example};

/// Add a vendor-specific field to a JSON object.
fn with_extension(mut value: Value) -> Value {
    value["x-vendor-nonsense"] = json!({"colour": "beige", "count": 3});
    value["some_future_is_04_field"] = json!(["not", "yet", "invented"]);
    value
}

/// Parse, add extensions, parse again, and assert the two agree.
fn unaffected_by_extensions<T>(raw: Value)
where
    T: serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let plain: T = serde_json::from_value(raw.clone()).expect("the example parses");
    let extended: T = serde_json::from_value(with_extension(raw))
        .expect("the example still parses with unknown fields");
    assert_eq!(
        plain, extended,
        "an unknown field changed the parsed resource"
    );
}

#[test]
fn a_node_tolerates_unknown_fields() {
    unaffected_by_extensions::<Node>(example(Spec::Is04, "nodeapi-self-get-200.json"));
}

#[test]
fn a_device_tolerates_unknown_fields() {
    unaffected_by_extensions::<Device>(
        example(Spec::Is04, "nodeapi-devices-get-200.json")[0].clone(),
    );
}

#[test]
fn a_sender_tolerates_unknown_fields() {
    unaffected_by_extensions::<Sender>(
        example(Spec::Is04, "nodeapi-senders-get-200.json")[0].clone(),
    );
}

#[test]
fn a_receiver_tolerates_unknown_fields() {
    unaffected_by_extensions::<Receiver>(
        example(Spec::Is04, "nodeapi-receivers-get-200.json")[0].clone(),
    );
}

#[test]
fn a_flow_tolerates_unknown_fields() {
    unaffected_by_extensions::<Flow>(example(Spec::Is04, "nodeapi-flows-get-200.json")[0].clone());
}

#[test]
fn a_source_tolerates_unknown_fields() {
    unaffected_by_extensions::<Source>(
        example(Spec::Is04, "nodeapi-sources-get-200.json")[0].clone(),
    );
}

#[test]
fn every_published_resource_tolerates_unknown_fields() {
    // The per-type tests above name the resources; this one makes sure no
    // element of any collection is quietly skipped.
    for (file, parse) in [
        (
            "nodeapi-devices-get-200.json",
            (|v| serde_json::from_value::<Device>(v).map(|_| ())) as fn(Value) -> _,
        ),
        ("nodeapi-senders-get-200.json", |v| {
            serde_json::from_value::<Sender>(v).map(|_| ())
        }),
        ("nodeapi-receivers-get-200.json", |v| {
            serde_json::from_value::<Receiver>(v).map(|_| ())
        }),
        ("nodeapi-flows-get-200.json", |v| {
            serde_json::from_value::<Flow>(v).map(|_| ())
        }),
        ("nodeapi-sources-get-200.json", |v| {
            serde_json::from_value::<Source>(v).map(|_| ())
        }),
    ] {
        for item in example(Spec::Is04, file).as_array().expect("an array") {
            parse(with_extension(item.clone()))
                .unwrap_or_else(|e| panic!("{file} with unknown fields: {e}"));
        }
    }
}

#[test]
fn a_nested_object_tolerates_unknown_fields_too() {
    // Extensions turn up inside `subscription` and `api`, not only at the top.
    let mut raw = example(Spec::Is04, "nodeapi-senders-get-200.json")[0].clone();
    let plain: Sender = serde_json::from_value(raw.clone()).expect("parses");
    raw["subscription"]["x-vendor-state"] = json!("beige");
    let extended: Sender = serde_json::from_value(raw).expect("parses with a nested extension");
    assert_eq!(plain, extended);
}
