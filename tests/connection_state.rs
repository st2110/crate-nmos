//! IS-04's `subscription` is where connection state comes from, and where the
//! protocol's vocabulary is exchanged for the project's.
//!
//! The case that matters is the one the bench produced: a Sender reporting
//! `{"active": true, "receiver_id": null}`. It is Transmitting, and it names
//! nobody — those are two facts, and conflating them is the mistake this whole
//! vocabulary exists to prevent.

// This file is test code in its entirety.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use nmos::{Receiver, Reception, Sender, Transmission};
use serde_json::json;
use support::{Spec, example};

fn sender_with_subscription(subscription: serde_json::Value) -> Sender {
    let mut raw = example(Spec::Is04, "nodeapi-senders-get-200.json")[0].clone();
    raw["subscription"] = subscription;
    serde_json::from_value(raw).expect("parses")
}

fn receiver_with_subscription(subscription: serde_json::Value) -> Receiver {
    let mut raw = example(Spec::Is04, "nodeapi-receivers-get-200.json")[0].clone();
    raw["subscription"] = subscription;
    serde_json::from_value(raw).expect("parses")
}

#[test]
fn a_sender_transmitting_to_nobody_is_transmitting_and_names_nobody() {
    // Exactly what the bench converter reports for its three live Senders.
    let sender = sender_with_subscription(json!({"active": true, "receiver_id": null}));

    assert_eq!(sender.transmission(), Transmission::Transmitting);
    assert!(
        sender.subscription.receiver_id.is_none(),
        "an empty receiver_id must not be turned into a pairing"
    );
}

#[test]
fn an_idle_sender_is_idle() {
    let sender = sender_with_subscription(json!({"active": false, "receiver_id": null}));
    assert_eq!(sender.transmission(), Transmission::Idle);
}

#[test]
fn a_sender_naming_its_receiver_keeps_that_name() {
    let sender = sender_with_subscription(json!({
        "active": true,
        "receiver_id": "3b8be755-08ff-452b-b217-c9151eb21193"
    }));
    assert_eq!(sender.transmission(), Transmission::Transmitting);
    assert_eq!(
        sender
            .subscription
            .receiver_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("3b8be755-08ff-452b-b217-c9151eb21193")
    );
}

#[test]
fn a_subscribed_receiver_naming_nobody_is_still_subscribed() {
    // The state and the pairing are independent: a Receiver that is taking a
    // stream but names no Sender is Subscribed with an unresolved edge, never
    // Unsubscribed.
    let receiver = receiver_with_subscription(json!({"active": true, "sender_id": null}));

    assert_eq!(receiver.reception(), Reception::Subscribed);
    assert!(receiver.subscription.sender_id.is_none());
}

#[test]
fn an_unsubscribed_receiver_is_unsubscribed() {
    let receiver = receiver_with_subscription(json!({"active": false, "sender_id": null}));
    assert_eq!(receiver.reception(), Reception::Unsubscribed);
}

#[test]
fn connection_state_comes_from_the_resource_tree_alone() {
    // No Connection API is involved in any of the above. This is what makes
    // state cost six requests per Node rather than one per resource.
    for sender in example(Spec::Is04, "nodeapi-senders-get-200.json")
        .as_array()
        .expect("an array")
    {
        let sender: Sender = serde_json::from_value(sender.clone()).expect("parses");
        let _: Transmission = sender.transmission();
    }
    for receiver in example(Spec::Is04, "nodeapi-receivers-get-200.json")
        .as_array()
        .expect("an array")
    {
        let receiver: Receiver = serde_json::from_value(receiver.clone()).expect("parses");
        let _: Reception = receiver.reception();
    }
}

#[test]
fn the_domain_states_read_as_the_glossary_writes_them() {
    assert_eq!(Transmission::Transmitting.to_string(), "transmitting");
    assert_eq!(Transmission::Idle.to_string(), "idle");
    assert_eq!(Reception::Subscribed.to_string(), "subscribed");
    assert_eq!(Reception::Unsubscribed.to_string(), "unsubscribed");
}

#[test]
fn a_subscription_survives_a_round_trip_through_the_protocol_flag() {
    for transmission in [Transmission::Transmitting, Transmission::Idle] {
        let subscription = nmos::SenderSubscription::new(transmission, None);
        let wire = serde_json::to_value(&subscription).expect("serializes");
        assert_eq!(wire["active"], json!(transmission.is_transmitting()));

        let back: nmos::SenderSubscription = serde_json::from_value(wire).expect("parses");
        assert_eq!(back.transmission(), transmission);
    }
}
