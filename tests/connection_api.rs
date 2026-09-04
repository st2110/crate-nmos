//! The IS-05 Connection API client.
//!
//! It reads transport parameters and nothing else. Connection state already
//! came from the resource tree — see `DESIGN.md`, "Two-tier reading" — and
//! this client exists only to answer the question IS-04 could not: which
//! address and port a stream actually uses, which is how a Receiver is paired
//! to a Sender when neither names the other.

// The HTTP clients live behind the `client` feature; so do their tests.
#![cfg(feature = "client")]
// This file is test code in its entirety.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::time::Duration;

use nmos::{ConnectionApiClient, ConnectionApiError, ResourceId};
use serde_json::json;
use support::fixture::FixtureNode;
use wiremock::ResponseTemplate;

const SENDER: &str = "3b8be755-08ff-452b-b217-c9151eb21193";
const RECEIVER: &str = "1eb53d65-ac83-441c-86f6-9b27df30ef0c";

fn client() -> ConnectionApiClient {
    ConnectionApiClient::builder()
        .request_timeout(Duration::from_millis(500))
        .connect_timeout(Duration::from_millis(500))
        .build()
        .expect("a client builds")
}

fn id(text: &str) -> ResourceId {
    text.parse().expect("a well-formed identifier")
}

// --- reading a Sender -------------------------------------------------------

#[tokio::test]
async fn a_senders_destination_is_read() {
    let node = FixtureNode::empty().await;
    node.serve_sender_active(
        "v1.1",
        SENDER,
        ResponseTemplate::new(200).set_body_json(FixtureNode::sender_active_example()),
    )
    .await;

    let transport = client()
        .fetch_sender_transport(&node.base_url(), "v1.1".parse().unwrap(), &id(SENDER))
        .await
        .expect("the fixture answers");

    // Two legs: this is ST 2022-7 seamless protection, not a duplicate.
    assert_eq!(transport.legs.len(), 2);
    let destination = transport.legs[0].destination().expect("a destination");
    assert_eq!(destination.address, "172.29.82.95");
    assert_eq!(destination.port, 5000);
    assert_eq!(destination.to_string(), "172.29.82.95:5000");
}

#[tokio::test]
async fn a_sender_whose_rtp_is_off_reports_no_destination() {
    let node = FixtureNode::empty().await;
    node.serve_sender_active(
        "v1.1",
        SENDER,
        ResponseTemplate::new(200).set_body_json(FixtureNode::sender_active_example()),
    )
    .await;
    let mut body = FixtureNode::sender_active_example();
    body["transport_params"] = json!([{"rtp_enabled": false}]);
    let node = FixtureNode::empty().await;
    node.serve_sender_active(
        "v1.1",
        SENDER,
        ResponseTemplate::new(200).set_body_json(body),
    )
    .await;

    let transport = client()
        .fetch_sender_transport(&node.base_url(), "v1.1".parse().unwrap(), &id(SENDER))
        .await
        .expect("answers");

    assert!(transport.legs[0].destination().is_none());
    assert!(!transport.legs[0].rtp_enabled);
}

#[tokio::test]
async fn an_auto_destination_is_not_a_destination() {
    // `auto` means the device chooses. Reporting it as an address would put a
    // pairing in the graph that does not exist.
    let mut body = FixtureNode::sender_active_example();
    body["transport_params"] = json!([{
        "destination_ip": "auto",
        "destination_port": "auto",
        "rtp_enabled": true
    }]);

    let node = FixtureNode::empty().await;
    node.serve_sender_active(
        "v1.1",
        SENDER,
        ResponseTemplate::new(200).set_body_json(body),
    )
    .await;

    let transport = client()
        .fetch_sender_transport(&node.base_url(), "v1.1".parse().unwrap(), &id(SENDER))
        .await
        .expect("answers");

    assert!(transport.legs[0].destination().is_none());
}

// --- reading a Receiver -----------------------------------------------------

#[tokio::test]
async fn a_receivers_stream_and_sender_are_read() {
    let node = FixtureNode::empty().await;
    node.serve_receiver_active(
        "v1.1",
        RECEIVER,
        ResponseTemplate::new(200).set_body_json(FixtureNode::receiver_active_example()),
    )
    .await;

    let transport = client()
        .fetch_receiver_transport(&node.base_url(), "v1.1".parse().unwrap(), &id(RECEIVER))
        .await
        .expect("answers");

    assert_eq!(
        transport.sender_id.as_ref().map(ResourceId::as_str),
        Some("5709255c-c0ae-4e1e-99a0-e872e83e48e0"),
        "the pairing a Receiver reports must survive the round trip"
    );

    let stream = transport.legs[0].stream().expect("a stream address");
    assert_eq!(stream.address, "232.250.98.80");
    assert_eq!(stream.port, 5010);
    assert!(
        transport.has_transport_file,
        "this Receiver was connected by SDP"
    );
}

#[tokio::test]
async fn a_receiver_naming_no_sender_still_reports_its_stream() {
    // The case the bench produced: a Receiver taking a stream whose Sender it
    // does not name. The stream address is the only way to pair it, which is
    // the whole reason this client exists.
    let mut body = FixtureNode::receiver_active_example();
    body["sender_id"] = json!(null);

    let node = FixtureNode::empty().await;
    node.serve_receiver_active(
        "v1.1",
        RECEIVER,
        ResponseTemplate::new(200).set_body_json(body),
    )
    .await;

    let transport = client()
        .fetch_receiver_transport(&node.base_url(), "v1.1".parse().unwrap(), &id(RECEIVER))
        .await
        .expect("answers");

    assert!(transport.sender_id.is_none());
    assert!(transport.legs[0].stream().is_some());
}

#[tokio::test]
async fn a_unicast_receiver_uses_its_source_address() {
    let mut body = FixtureNode::receiver_active_example();
    body["transport_params"] = json!([{
        "source_ip": "172.29.226.25",
        "interface_ip": "172.23.19.35",
        "destination_port": 5010,
        "rtp_enabled": true
    }]);

    let node = FixtureNode::empty().await;
    node.serve_receiver_active(
        "v1.1",
        RECEIVER,
        ResponseTemplate::new(200).set_body_json(body),
    )
    .await;

    let transport = client()
        .fetch_receiver_transport(&node.base_url(), "v1.1".parse().unwrap(), &id(RECEIVER))
        .await
        .expect("answers");

    let stream = transport.legs[0].stream().expect("a stream address");
    assert_eq!(stream.address, "172.29.226.25");
    assert_eq!(stream.port, 5010);
}

// --- versions and failure ---------------------------------------------------

#[tokio::test]
async fn a_node_serving_only_v1_0_is_addressed_at_v1_0() {
    let node = FixtureNode::empty().await;
    node.serve_sender_active(
        "v1.0",
        SENDER,
        ResponseTemplate::new(200).set_body_json(FixtureNode::sender_active_example()),
    )
    .await;

    let transport = client()
        .fetch_sender_transport(&node.base_url(), "v1.0".parse().unwrap(), &id(SENDER))
        .await
        .expect("answers at v1.0");
    assert_eq!(transport.legs.len(), 2);
}

#[tokio::test]
async fn an_unreachable_connection_api_is_a_named_error() {
    let error = client()
        .fetch_sender_transport("http://127.0.0.1:1", "v1.1".parse().unwrap(), &id(SENDER))
        .await
        .expect_err("nothing is listening");
    assert!(
        matches!(error, ConnectionApiError::Unreachable { .. }),
        "got {error:?}"
    );
}

#[tokio::test]
async fn an_http_error_from_the_connection_api_keeps_its_status() {
    let node = FixtureNode::empty().await;
    node.serve_sender_active("v1.1", SENDER, ResponseTemplate::new(404))
        .await;

    let error = client()
        .fetch_sender_transport(&node.base_url(), "v1.1".parse().unwrap(), &id(SENDER))
        .await
        .expect_err("404 is not a transport");

    match error {
        ConnectionApiError::Status { status, .. } => assert_eq!(status, 404),
        other => panic!("expected a status error, got {other:?}"),
    }
}

#[tokio::test]
async fn a_malformed_transport_says_so() {
    let node = FixtureNode::empty().await;
    node.serve_sender_active(
        "v1.1",
        SENDER,
        ResponseTemplate::new(200).set_body_string("<html>login</html>"),
    )
    .await;

    let error = client()
        .fetch_sender_transport(&node.base_url(), "v1.1".parse().unwrap(), &id(SENDER))
        .await
        .expect_err("a login page is not a transport");
    assert!(
        matches!(error, ConnectionApiError::Malformed { .. }),
        "got {error:?}"
    );
}

#[tokio::test]
async fn a_timeout_from_the_connection_api_is_a_named_error() {
    let node = FixtureNode::empty().await;
    node.serve_sender_active(
        "v1.1",
        SENDER,
        ResponseTemplate::new(200).set_delay(Duration::from_secs(30)),
    )
    .await;

    let error = client()
        .fetch_sender_transport(&node.base_url(), "v1.1".parse().unwrap(), &id(SENDER))
        .await
        .expect_err("the fixture never answers");
    assert!(
        matches!(error, ConnectionApiError::Timeout { .. }),
        "got {error:?}"
    );
}

// --- the two tiers are independent ------------------------------------------

#[tokio::test]
async fn a_node_without_a_reachable_connection_api_still_has_valid_state() {
    // The point of reading state from the resource tree: a Node whose IS-05 is
    // firewalled off, broken, or simply absent is still a Node whose Senders
    // are known to be Transmitting or Idle. Only the destinations are missing.
    use nmos::{NodeApiClient, Transmission};

    let node = FixtureNode::serving("v1.3").await;

    let tree = NodeApiClient::builder()
        .request_timeout(Duration::from_millis(500))
        .build()
        .expect("a client builds")
        .fetch_tree(&node.base_url(), &["v1.3".parse().unwrap()])
        .await
        .expect("the Node API answers");

    let sender = tree.senders.first().expect("at least one Sender");
    assert_eq!(sender.transmission(), Transmission::Transmitting);

    // The fixture serves no Connection API at all.
    let error = client()
        .fetch_sender_transport(&node.base_url(), "v1.1".parse().unwrap(), &sender.core.id)
        .await
        .expect_err("this fixture has no Connection API");
    assert!(
        matches!(error, ConnectionApiError::Status { status: 404, .. }),
        "got {error:?}"
    );

    // And the state read from the tree is untouched by that failure.
    assert_eq!(sender.transmission(), Transmission::Transmitting);
    for receiver in &tree.receivers {
        let _ = receiver.reception();
    }
}
