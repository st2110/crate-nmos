//! The IS-05 documents a controller stages and a Node applies.
//!
//! These are checked against the vendored `receiver-stage-schema.json` and
//! `sender-stage-schema.json` in both directions, because both ends of the
//! protocol read them: a controller writes what a Node must accept.

// This file is test code in its entirety.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::net::{IpAddr, Ipv4Addr};

use nmos::Version;
use nmos::is05::{
    Activation, ActivationMode, Constraint, Param, ReceiverRtpParams, ReceiverStagedPatch,
    SenderStagedPatch, TRANSPORT_FILE_TYPE, TransportFile,
};
use serde_json::json;
use support::{Spec, assert_valid};

#[test]
fn a_parameter_has_four_states_because_a_patch_needs_four() {
    // Absent means "leave it", null means "clear it", "auto" means "you decide",
    // and a value means "use this". Two states cannot express a PATCH.
    let patch: ReceiverRtpParams =
        serde_json::from_value(json!({"multicast_ip": null, "destination_port": "auto"})).unwrap();

    assert_eq!(patch.multicast_ip, Param::Null);
    assert_eq!(patch.destination_port, Param::Auto);
    assert_eq!(
        patch.source_ip,
        Param::Absent,
        "an unmentioned field is absent"
    );
    assert_eq!(patch.rtp_enabled, Param::Absent);
}

#[test]
fn a_set_parameter_carries_its_value() {
    let patch: ReceiverRtpParams = serde_json::from_value(
        json!({"multicast_ip": "239.1.1.1", "destination_port": 5004, "rtp_enabled": true}),
    )
    .unwrap();

    assert_eq!(
        patch.multicast_ip,
        Param::Set(IpAddr::V4(Ipv4Addr::new(239, 1, 1, 1)))
    );
    assert_eq!(patch.destination_port, Param::Set(5004));
    assert_eq!(patch.rtp_enabled, Param::Set(true));
}

#[test]
fn absent_and_null_are_not_the_same_thing() {
    // The distinction this whole type exists for: one leaves the value alone,
    // the other clears it.
    assert_eq!(Param::<u16>::Absent.value(), None);
    assert_eq!(Param::<u16>::Null.value(), Some(None));
    assert_eq!(Param::<u16>::Auto.value(), Some(None));
    assert_eq!(Param::Set(5004u16).value(), Some(Some(5004)));
}

#[test]
fn the_boundary_ports_survive_the_round_trip() {
    for port in [0u16, 1, 5004, u16::MAX] {
        let value = serde_json::to_value(Param::Set(port)).unwrap();
        assert_eq!(value, json!(port));
        let back: Param<u16> = serde_json::from_value(value).unwrap();
        assert_eq!(back, Param::Set(port));
    }
}

#[test]
fn auto_serialises_back_as_the_string_the_schema_names() {
    assert_eq!(
        serde_json::to_value(Param::<u16>::Auto).unwrap(),
        json!("auto")
    );
    assert_eq!(
        serde_json::to_value(Param::<u16>::Null).unwrap(),
        json!(null)
    );
}

#[test]
fn an_unknown_parameter_is_refused_rather_than_swallowed() {
    // A silently ignored parameter is a controller showing a connection that
    // does not exist. The contract is AMWA's and does not grow on the fly.
    let refused: Result<ReceiverRtpParams, _> =
        serde_json::from_value(json!({"multicast_ip": "239.1.1.1", "invented": 1}));
    assert!(refused.is_err(), "unknown fields must be rejected");
}

#[test]
fn a_receiver_patch_validates_against_the_published_schema() {
    let patch = json!({
        "sender_id": "6c1b8b8a-1f9a-4b3e-9e3a-6b8c1a2d3e4f",
        "master_enable": true,
        "activation": {"mode": "activate_immediate"},
        "transport_file": {"data": "v=0\r\n", "type": TRANSPORT_FILE_TYPE},
        "transport_params": [{"multicast_ip": "239.1.1.1", "destination_port": 5004}]
    });
    let parsed: ReceiverStagedPatch = serde_json::from_value(patch.clone()).unwrap();
    assert_eq!(parsed.master_enable, Some(true));
    assert_valid(Spec::Is05, "receiver-stage-schema.json", &patch);
}

#[test]
fn a_sender_patch_names_a_receiver_not_a_sender() {
    let patch = json!({
        "receiver_id": "6c1b8b8a-1f9a-4b3e-9e3a-6b8c1a2d3e4f",
        "master_enable": false,
        "transport_params": [{"destination_ip": "239.2.2.2", "destination_port": "auto"}]
    });
    let parsed: SenderStagedPatch = serde_json::from_value(patch.clone()).unwrap();
    assert_eq!(parsed.master_enable, Some(false));
    assert_valid(Spec::Is05, "sender-stage-schema.json", &patch);
}

#[test]
fn the_activation_modes_are_the_three_the_specification_names() {
    for (mode, wire) in [
        (ActivationMode::ActivateImmediate, "activate_immediate"),
        (
            ActivationMode::ActivateScheduledAbsolute,
            "activate_scheduled_absolute",
        ),
        (
            ActivationMode::ActivateScheduledRelative,
            "activate_scheduled_relative",
        ),
    ] {
        assert_eq!(serde_json::to_value(mode).unwrap(), json!(wire));
        let back: ActivationMode = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(back, mode);
    }
}

#[test]
fn a_settled_activation_keeps_its_time_and_drops_its_mode() {
    // The mode is the request; once it has happened only the time remains, and
    // a Node that keeps answering "activate_immediate" is claiming a pending
    // activation it does not have.
    let at = Version::new(1234567890, 0);
    let activation = Activation::immediate(at);
    assert_eq!(activation.mode, Some(ActivationMode::ActivateImmediate));
    assert_eq!(activation.activation_time, Some(at));

    let settled = activation.settled();
    assert_eq!(settled.mode, None);
    assert_eq!(settled.activation_time, Some(at));
}

#[test]
fn a_transport_file_without_data_has_no_type_either() {
    // Announcing "application/sdp" over an absent document describes nothing.
    let empty = TransportFile::sdp(None);
    assert_eq!(empty.data, None);
    assert_eq!(empty.file_type, None);

    let present = TransportFile::sdp(Some("v=0\r\n".to_owned()));
    assert_eq!(present.file_type.as_deref(), Some(TRANSPORT_FILE_TYPE));
}

#[test]
fn a_constraint_omits_the_bounds_it_does_not_set() {
    let bare = serde_json::to_value(Constraint::default()).unwrap();
    assert_eq!(bare, json!({}), "an unconstrained parameter says nothing");

    let port = serde_json::to_value(Constraint::port()).unwrap();
    assert_eq!(port, json!({"minimum": 1, "maximum": 65535}));

    let one = serde_json::to_value(Constraint::one_of(["10.0.0.1"])).unwrap();
    assert_eq!(one, json!({"enum": ["10.0.0.1"]}));
    assert_valid(Spec::Is05, "constraint-schema.json", &one);
}

#[test]
fn an_empty_patch_changes_nothing_and_is_still_a_patch() {
    let parsed: ReceiverStagedPatch = serde_json::from_value(json!({})).unwrap();
    assert_eq!(parsed.master_enable, None);
    assert!(parsed.transport_params.is_none());
    assert_eq!(parsed.sender_id, Param::Absent);
}

#[test]
fn a_null_sender_id_disconnects_rather_than_leaving_it_alone() {
    let parsed: ReceiverStagedPatch = serde_json::from_value(json!({"sender_id": null})).unwrap();
    assert_eq!(parsed.sender_id, Param::Null);
    assert_eq!(parsed.sender_id.value(), Some(None));
}
