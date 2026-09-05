//! Transport parameters come in families, and IS-05 does not say which.
//!
//! The documents carry no discriminator: whether a leg is RTP or MQTT is known
//! from the Sender's or Receiver's `transport` field, which lives in a different
//! API. So the family is something the caller states, and these tests fix what
//! happens when it states it rightly, wrongly, or not at all.

// This file is test code in its entirety.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use nmos::is05::{
    Param, ReceiverMqttParams, ReceiverRtpParams, ReceiverStagedPatch, ReceiverWebsocketParams,
    SenderRtpParams, SenderStagedPatch, SenderWebsocketParams, UnknownParams,
};
use serde_json::json;
use support::{Spec, assert_valid};

#[test]
fn rtp_is_what_a_staged_patch_assumes() {
    // The default family, because it is the one ST 2110 uses.
    let patch: ReceiverStagedPatch =
        serde_json::from_value(json!({"transport_params": [{"multicast_ip": "239.1.1.1"}]}))
            .unwrap();
    let legs = patch.transport_params.unwrap();
    assert_eq!(
        legs[0].multicast_ip,
        Param::Set("239.1.1.1".parse().unwrap())
    );
}

#[test]
fn a_websocket_leg_parses_when_the_caller_says_so() {
    let patch: ReceiverStagedPatch<ReceiverWebsocketParams> = serde_json::from_value(json!({
        "transport_params": [{"connection_uri": "ws://10.0.0.1:8080/x", "connection_authorization": false}]
    }))
    .unwrap();
    let legs = patch.transport_params.unwrap();
    assert_eq!(
        legs[0].connection_uri,
        Param::Set("ws://10.0.0.1:8080/x".to_owned())
    );
}

#[test]
fn authorization_is_either_a_boolean_or_a_string() {
    // The schema allows both, so the type has to. A `bool` here would reject
    // half of what the specification permits.
    for value in [json!(false), json!("Bearer")] {
        let patch: ReceiverStagedPatch<ReceiverWebsocketParams> = serde_json::from_value(
            json!({"transport_params": [{"connection_authorization": value}]}),
        )
        .unwrap();
        let legs = patch.transport_params.unwrap();
        assert!(matches!(legs[0].connection_authorization, Param::Set(_)));
    }
}

#[test]
fn an_mqtt_leg_carries_its_broker() {
    let patch: ReceiverStagedPatch<ReceiverMqttParams> = serde_json::from_value(json!({
        "transport_params": [{
            "source_host": "broker.local",
            "source_port": 1883,
            "broker_protocol": "mqtt",
            "broker_topic": "x-nmos/events"
        }]
    }))
    .unwrap();
    let legs = patch.transport_params.unwrap();
    assert_eq!(legs[0].source_port, Param::Set(1883));
    assert_eq!(legs[0].broker_topic, Param::Set("x-nmos/events".to_owned()));
}

#[test]
fn a_family_still_refuses_what_does_not_belong_to_it() {
    // Strictness inside the family is how a Node answers 400 to nonsense, and
    // generalising must not cost it.
    let refused: Result<ReceiverStagedPatch, _> =
        serde_json::from_value(json!({"transport_params": [{"connection_uri": "ws://x"}]}));
    assert!(refused.is_err(), "a websocket leg is not an RTP leg");

    let also_refused: Result<ReceiverStagedPatch<ReceiverWebsocketParams>, _> =
        serde_json::from_value(json!({"transport_params": [{"multicast_ip": "239.1.1.1"}]}));
    assert!(also_refused.is_err(), "an RTP leg is not a websocket leg");
}

#[test]
fn an_unfamiliar_transport_is_carried_rather_than_refused() {
    // A controller meeting equipment whose transport it does not model should
    // still be able to read and hand back what it was given. Refusing would
    // make this crate the reason an operator cannot see their device.
    let document = json!({"transport_params": [{
        "mxl_domain": "urn:example:domain",
        "mxl_flow_id": "1a2b3c",
        "rtp_enabled": true
    }]});
    let patch: ReceiverStagedPatch<UnknownParams> =
        serde_json::from_value(document.clone()).unwrap();
    let legs = patch.transport_params.as_ref().unwrap();
    assert_eq!(
        legs[0].get("mxl_domain"),
        Some(&json!("urn:example:domain"))
    );

    // And nothing is lost on the way back out.
    let back = serde_json::to_value(&patch).unwrap();
    assert_eq!(back["transport_params"], document["transport_params"]);
}

#[test]
fn a_sender_patch_is_generic_the_same_way() {
    let patch: SenderStagedPatch<SenderWebsocketParams> = serde_json::from_value(json!({
        "transport_params": [{"connection_uri": "ws://10.0.0.2:9000/y"}]
    }))
    .unwrap();
    assert!(patch.transport_params.is_some());

    let rtp: SenderStagedPatch<SenderRtpParams> = serde_json::from_value(json!({
        "transport_params": [{"destination_ip": "239.2.2.2", "destination_port": 5004}]
    }))
    .unwrap();
    let legs = rtp.transport_params.unwrap();
    assert_eq!(legs[0].destination_port, Param::Set(5004));
}

#[test]
fn what_the_families_serialise_still_satisfies_the_published_schema() {
    let websocket = json!({
        "master_enable": true,
        "transport_params": [{"connection_uri": "ws://10.0.0.1:8080/x"}]
    });
    assert_valid(Spec::Is05, "receiver-stage-schema.json", &websocket);

    let mqtt = json!({
        "master_enable": true,
        "transport_params": [{"source_host": "broker.local", "source_port": 1883}]
    });
    assert_valid(Spec::Is05, "receiver-stage-schema.json", &mqtt);
}

#[test]
fn an_empty_leg_is_legal_in_every_family() {
    // "Change nothing about this leg" is a thing a controller may say.
    let rtp: ReceiverRtpParams = serde_json::from_value(json!({})).unwrap();
    assert_eq!(rtp.multicast_ip, Param::Absent);
    let ws: ReceiverWebsocketParams = serde_json::from_value(json!({})).unwrap();
    assert_eq!(ws.connection_uri, Param::Absent);
    let unknown: UnknownParams = serde_json::from_value(json!({})).unwrap();
    assert!(unknown.is_empty());
}
