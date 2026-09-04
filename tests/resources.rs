//! Every resource this project consumes, checked against the published contract
//! in both directions: the specification's own examples must parse into our
//! types, and what our types serialize must satisfy the schema.

// This file is test code in its entirety.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use nmos::{Device, Flow, Format, MediaType, Node, Receiver, ReceiverCaps, Sender, Source};
use serde_json::{Value, json};
use support::{Spec, assert_valid, example};

/// Parse a collection example into `T`, asserting it is not empty.
fn collection<T: serde::de::DeserializeOwned>(file: &str) -> Vec<T> {
    let value = example(Spec::Is04, file);
    let items = value.as_array().expect("a collection is an array").clone();
    assert!(!items.is_empty(), "{file} has no elements to exercise");
    items
        .into_iter()
        .map(|item| {
            serde_json::from_value(item.clone()).unwrap_or_else(|e| panic!("{file}: {e}\n{item:#}"))
        })
        .collect()
}

/// Serialize and check against a vendored schema.
fn round_trip<T: serde::Serialize>(value: &T, schema: &str) -> Value {
    let serialized = serde_json::to_value(value).expect("serializes");
    assert_valid(Spec::Is04, schema, &serialized);
    serialized
}

// --- Node -------------------------------------------------------------------

#[test]
fn the_published_node_parses_and_round_trips() {
    let raw = example(Spec::Is04, "nodeapi-self-get-200.json");
    let node: Node = serde_json::from_value(raw).expect("the Node example parses");

    assert!(!node.core.label.is_empty());
    assert!(!node.api.versions.is_empty());
    assert!(!node.api.endpoints.is_empty());

    round_trip(&node, "node.json");
}

#[test]
fn a_nodes_api_endpoints_carry_host_port_and_protocol() {
    let raw = example(Spec::Is04, "nodeapi-self-get-200.json");
    let node: Node = serde_json::from_value(raw).expect("parses");
    let endpoint = node.api.endpoints.first().expect("at least one endpoint");
    assert!(!endpoint.host.is_empty());
    assert!(endpoint.port > 0);
}

// --- Device -----------------------------------------------------------------

#[test]
fn the_published_devices_parse_and_round_trip() {
    let devices: Vec<Device> = collection("nodeapi-devices-get-200.json");
    for device in &devices {
        round_trip(device, "device.json");
    }
}

#[test]
fn a_devices_controls_are_available_for_finding_the_connection_api() {
    // The IS-05 base URL is advertised as a Device control, so this is not
    // decoration: it is how the second tier of the fetch finds its endpoint.
    let devices: Vec<Device> = collection("nodeapi-devices-get-200.json");
    let has_control = devices.iter().any(|d| !d.controls.is_empty());
    assert!(
        has_control,
        "the published Device example carries no controls"
    );
}

// --- Sender -----------------------------------------------------------------

#[test]
fn the_published_senders_parse_and_round_trip() {
    let senders: Vec<Sender> = collection("nodeapi-senders-get-200.json");
    for sender in &senders {
        round_trip(sender, "sender.json");
    }
}

#[test]
fn a_sender_without_a_flow_parses() {
    // `flow_id` is null when no Flow is internally routed to the Sender.
    let mut raw = example(Spec::Is04, "nodeapi-senders-get-200.json")[0].clone();
    raw["flow_id"] = Value::Null;
    let sender: Sender = serde_json::from_value(raw).expect("parses");
    assert!(sender.flow_id.is_none());
}

// --- Receiver ---------------------------------------------------------------

#[test]
fn the_published_receivers_parse_and_round_trip() {
    let receivers: Vec<Receiver> = collection("nodeapi-receivers-get-200.json");
    for receiver in &receivers {
        round_trip(receiver, "receiver.json");
    }
}

#[test]
fn a_receiver_declares_the_media_types_it_accepts() {
    let receivers: Vec<Receiver> = collection("nodeapi-receivers-get-200.json");
    let receiver = receivers.first().expect("at least one Receiver");
    assert!(
        !receiver.caps.media_types().is_empty(),
        "a Receiver row is unusable without the media types it accepts"
    );
}

#[test]
fn receiver_variants_follow_the_declared_format() {
    for (format, expected) in [
        ("urn:x-nmos:format:video", "video"),
        ("urn:x-nmos:format:audio", "audio"),
        ("urn:x-nmos:format:data", "data"),
        ("urn:x-nmos:format:mux", "mux"),
    ] {
        let mut raw = example(Spec::Is04, "nodeapi-receivers-get-200.json")[0].clone();
        raw["format"] = json!(format);
        raw["caps"] = json!({"media_types": ["video/raw"]});
        let receiver: Receiver = serde_json::from_value(raw).expect("parses");
        let actual = match receiver.caps {
            ReceiverCaps::Video { .. } => "video",
            ReceiverCaps::Audio { .. } => "audio",
            ReceiverCaps::Data { .. } => "data",
            ReceiverCaps::Mux { .. } => "mux",
        };
        assert_eq!(
            actual, expected,
            "{format} selected the wrong Receiver variant"
        );
    }
}

#[test]
fn a_receiver_with_an_unknown_format_is_rejected_not_guessed() {
    let mut raw = example(Spec::Is04, "nodeapi-receivers-get-200.json")[0].clone();
    raw["format"] = json!("urn:x-nmos:format:hologram");
    assert!(serde_json::from_value::<Receiver>(raw).is_err());
}

// --- Flow -------------------------------------------------------------------

#[test]
fn the_published_flows_parse_and_round_trip() {
    let flows: Vec<Flow> = collection("nodeapi-flows-get-200.json");
    for flow in &flows {
        round_trip(flow, "flow.json");
    }
}

/// The rule that picks a Flow variant, stated as data so the test reads as the
/// specification does: format first, then media type.
const FLOW_VARIANTS: &[(&str, &str, &str)] = &[
    ("urn:x-nmos:format:video", "video/raw", "video raw"),
    ("urn:x-nmos:format:video", "video/H264", "video coded"),
    ("urn:x-nmos:format:video", "video/vc2", "video coded"),
    ("urn:x-nmos:format:audio", "audio/L24", "audio raw"),
    ("urn:x-nmos:format:audio", "audio/L16", "audio raw"),
    ("urn:x-nmos:format:audio", "audio/AAC", "audio coded"),
    ("urn:x-nmos:format:data", "video/smpte291", "sdianc data"),
    ("urn:x-nmos:format:data", "application/json", "json data"),
    ("urn:x-nmos:format:data", "application/octet-stream", "data"),
    ("urn:x-nmos:format:mux", "video/SMPTE2022-6", "mux"),
];

fn flow_variant_name(flow: &Flow) -> &'static str {
    match flow {
        Flow::VideoRaw { .. } => "video raw",
        Flow::VideoCoded { .. } => "video coded",
        Flow::AudioRaw { .. } => "audio raw",
        Flow::AudioCoded { .. } => "audio coded",
        Flow::SdiAncData { .. } => "sdianc data",
        Flow::JsonData { .. } => "json data",
        Flow::Data { .. } => "data",
        Flow::Mux { .. } => "mux",
    }
}

#[test]
fn every_flow_variant_is_selected_by_format_and_media_type() {
    for (format, media_type, expected) in FLOW_VARIANTS {
        let raw = json!({
            "id": "3b8be755-08ff-452b-b217-c9151eb21193",
            "version": "1441700172:318426300",
            "label": "SDI 1",
            "description": "",
            "tags": {},
            "source_id": "6cddb0fb-9b6d-4f88-8d17-9b6a3a1cd4f6",
            "device_id": "9126cc2f-4c26-4c9b-a6cd-93c4381c9be5",
            "parents": [],
            "format": format,
            "media_type": media_type,
            "frame_width": 1920,
            "frame_height": 1080,
            "colorspace": "BT709",
            "components": [{"name": "Y", "width": 1920, "height": 1080, "bit_depth": 10}],
            "sample_rate": {"numerator": 48000},
            "bit_depth": 24
        });
        let flow: Flow =
            serde_json::from_value(raw).unwrap_or_else(|e| panic!("{format} {media_type}: {e}"));
        assert_eq!(
            flow_variant_name(&flow),
            *expected,
            "{format} with {media_type} selected the wrong variant"
        );
    }
}

#[test]
fn a_flow_with_an_unknown_format_is_rejected_not_guessed() {
    let raw = json!({
        "id": "3b8be755-08ff-452b-b217-c9151eb21193",
        "version": "1441700172:318426300",
        "label": "",
        "description": "",
        "tags": {},
        "source_id": "6cddb0fb-9b6d-4f88-8d17-9b6a3a1cd4f6",
        "device_id": "9126cc2f-4c26-4c9b-a6cd-93c4381c9be5",
        "parents": [],
        "format": "urn:x-nmos:format:hologram",
        "media_type": "hologram/raw"
    });
    assert!(serde_json::from_value::<Flow>(raw).is_err());
}

#[test]
fn a_flow_reports_the_media_type_that_tells_two_rows_apart() {
    // The bench converter presents three Senders all labelled `SDI 1`, told
    // apart only by this.
    let flows: Vec<Flow> = collection("nodeapi-flows-get-200.json");
    for flow in &flows {
        assert!(!flow.media_type().as_str().is_empty());
    }
}

// --- Source -----------------------------------------------------------------

#[test]
fn the_published_sources_parse_and_round_trip() {
    let sources: Vec<Source> = collection("nodeapi-sources-get-200.json");
    for source in &sources {
        round_trip(source, "source.json");
    }
}

// --- media types and formats ------------------------------------------------

#[test]
fn a_media_type_is_a_type_and_a_subtype() {
    let media_type: MediaType = "video/raw".parse().expect("parses");
    assert_eq!(media_type.as_str(), "video/raw");
    assert_eq!(media_type.to_string(), "video/raw");

    for rejected in ["", "video", "video/", "/raw", "video/ raw", "a/b/c"] {
        assert!(
            rejected.parse::<MediaType>().is_err(),
            "`{rejected}` must not parse as a media type"
        );
    }
}

#[test]
fn a_format_round_trips_through_its_urn() {
    for (urn, format) in [
        ("urn:x-nmos:format:video", Format::Video),
        ("urn:x-nmos:format:audio", Format::Audio),
        ("urn:x-nmos:format:data", Format::Data),
        ("urn:x-nmos:format:mux", Format::Mux),
    ] {
        assert_eq!(urn.parse::<Format>().expect("parses"), format);
        assert_eq!(format.to_string(), urn);
    }
    assert!("urn:x-nmos:format:hologram".parse::<Format>().is_err());
}
