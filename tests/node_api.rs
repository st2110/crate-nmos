//! The IS-04 Node API client, exercised against a fixture Node built from the
//! published examples.
//!
//! Every failure a real plant produces is here too, because the specs require
//! each of them to be shown against the Node that caused it rather than
//! swallowed: refused connections, timeouts, HTTP errors, HTML where JSON was
//! expected, and JSON that stops halfway.

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

use nmos::{ApiVersion, NodeApiClient, NodeApiError};
use serde_json::json;
use support::fixture::FixtureNode;
use wiremock::ResponseTemplate;

/// A client with timeouts short enough that a stalled fixture does not stall
/// the test suite.
fn client() -> NodeApiClient {
    NodeApiClient::builder()
        .request_timeout(Duration::from_millis(500))
        .connect_timeout(Duration::from_millis(500))
        .build()
        .expect("a client builds")
}

// --- the happy path ---------------------------------------------------------

#[tokio::test]
async fn a_whole_resource_tree_is_read() {
    let node = FixtureNode::serving("v1.3").await;
    let tree = client()
        .fetch_tree(&node.base_url(), &["v1.3".parse().expect("a version")])
        .await
        .expect("the fixture answers");

    assert!(!tree.node.core.label.is_empty());
    assert!(!tree.devices.is_empty());
    assert!(!tree.senders.is_empty());
    assert!(!tree.receivers.is_empty());
    assert!(!tree.flows.is_empty());
    assert!(!tree.sources.is_empty());
    assert_eq!(
        tree.version,
        "v1.3".parse::<ApiVersion>().expect("a version")
    );
}

#[tokio::test]
async fn a_node_exposing_no_devices_is_not_an_error() {
    let node = FixtureNode::empty().await;
    node.serve_all("v1.3").await;
    node.serve(
        "v1.3",
        "devices",
        ResponseTemplate::new(200).set_body_json(json!([])),
    )
    .await;

    let tree = client()
        .fetch_tree(&node.base_url(), &["v1.3".parse().expect("a version")])
        .await
        .expect("an empty Node is a Node");

    assert!(tree.devices.is_empty());
}

#[tokio::test]
async fn requests_to_one_node_are_made_in_sequence() {
    // Small equipment answers one request at a time; the specs require the
    // fetch of a single Node not to flood it.
    let node = FixtureNode::serving("v1.3").await;
    let _ = client()
        .fetch_tree(&node.base_url(), &["v1.3".parse().expect("a version")])
        .await
        .expect("answers");

    assert_eq!(
        node.requests().await,
        6,
        "one request per collection, no more"
    );
}

// --- version negotiation ----------------------------------------------------

#[tokio::test]
async fn the_highest_supported_version_is_used() {
    let node = FixtureNode::empty().await;
    node.serve_all("v1.3").await;
    node.serve_all("v1.2").await;

    let tree = client()
        .fetch_tree(
            &node.base_url(),
            &[
                "v1.0".parse().unwrap(),
                "v1.2".parse().unwrap(),
                "v1.3".parse().unwrap(),
            ],
        )
        .await
        .expect("answers");

    assert_eq!(tree.version, "v1.3".parse::<ApiVersion>().unwrap());
    for path in node.request_paths().await {
        assert!(
            path.contains("/v1.3/"),
            "{path} is not the negotiated version"
        );
    }
}

#[tokio::test]
async fn a_node_offering_only_v1_2_is_addressed_at_v1_2() {
    let node = FixtureNode::empty().await;
    node.serve_all("v1.2").await;

    let tree = client()
        .fetch_tree(
            &node.base_url(),
            &["v1.0".parse().unwrap(), "v1.2".parse().unwrap()],
        )
        .await
        .expect("answers");

    assert_eq!(tree.version, "v1.2".parse::<ApiVersion>().unwrap());
}

#[tokio::test]
async fn a_node_offering_no_supported_version_names_what_it_offered() {
    let node = FixtureNode::empty().await;
    let offered: Vec<_> = ["v1.0", "v1.1"]
        .iter()
        .map(|v| v.parse().unwrap())
        .collect();

    let error = client()
        .fetch_tree(&node.base_url(), &offered)
        .await
        .expect_err("v1.0 and v1.1 are not supported");

    match error {
        NodeApiError::UnsupportedVersion { offered, supported } => {
            assert_eq!(offered, vec!["v1.0".to_owned(), "v1.1".to_owned()]);
            assert!(supported.contains(&"v1.3".to_owned()));
        }
        other => panic!("expected an unsupported-version error, got {other:?}"),
    }
}

#[tokio::test]
async fn a_node_advertising_no_versions_at_all_is_unsupported() {
    let node = FixtureNode::empty().await;
    let error = client()
        .fetch_tree(&node.base_url(), &[])
        .await
        .expect_err("a Node advertising nothing cannot be addressed");
    assert!(matches!(error, NodeApiError::UnsupportedVersion { .. }));
}

#[tokio::test]
async fn an_unparseable_version_is_rejected_rather_than_guessed() {
    for text in ["", "1.3", "v1", "vX.Y", "v1.3.1", "v-1.3"] {
        assert!(
            text.parse::<ApiVersion>().is_err(),
            "`{text}` must not parse as an API version"
        );
    }
    let version: ApiVersion = "v1.3".parse().expect("parses");
    assert_eq!(version.to_string(), "v1.3");
    assert!("v1.2".parse::<ApiVersion>().unwrap() < version);
}

// --- failures ---------------------------------------------------------------

#[tokio::test]
async fn a_refused_connection_is_a_named_error() {
    // Port 1 on the loopback interface: nothing listens there.
    let error = client()
        .fetch_tree("http://127.0.0.1:1", &["v1.3".parse().unwrap()])
        .await
        .expect_err("nothing is listening");

    assert!(
        matches!(error, NodeApiError::Unreachable { .. }),
        "expected an unreachable error, got {error:?}"
    );
}

#[tokio::test]
async fn a_timeout_is_a_named_error() {
    let node = FixtureNode::empty().await;
    node.serve(
        "v1.3",
        "self",
        ResponseTemplate::new(200).set_delay(Duration::from_secs(30)),
    )
    .await;

    let error = client()
        .fetch_tree(&node.base_url(), &["v1.3".parse().unwrap()])
        .await
        .expect_err("the fixture never answers in time");

    assert!(
        matches!(error, NodeApiError::Timeout { .. }),
        "expected a timeout, got {error:?}"
    );
}

#[tokio::test]
async fn an_http_error_status_is_reported_with_its_status() {
    let node = FixtureNode::empty().await;
    node.serve("v1.3", "self", ResponseTemplate::new(500)).await;

    let error = client()
        .fetch_tree(&node.base_url(), &["v1.3".parse().unwrap()])
        .await
        .expect_err("500 is not a resource tree");

    match error {
        NodeApiError::Status { status, .. } => assert_eq!(status, 500),
        other => panic!("expected a status error, got {other:?}"),
    }
}

#[tokio::test]
async fn html_where_json_was_expected_says_so() {
    let node = FixtureNode::empty().await;
    node.serve(
        "v1.3",
        "self",
        ResponseTemplate::new(200)
            .set_body_string("<html><body>Router login</body></html>")
            .insert_header("content-type", "text/html"),
    )
    .await;

    let error = client()
        .fetch_tree(&node.base_url(), &["v1.3".parse().unwrap()])
        .await
        .expect_err("a login page is not a Node");

    assert!(
        matches!(error, NodeApiError::Malformed { .. }),
        "expected a malformed-response error, got {error:?}"
    );
}

#[tokio::test]
async fn truncated_json_says_so() {
    let node = FixtureNode::empty().await;
    node.serve(
        "v1.3",
        "self",
        ResponseTemplate::new(200)
            .set_body_string(r#"{"id": "3b8be755-08ff-452b-b217-c915"#)
            .insert_header("content-type", "application/json"),
    )
    .await;

    let error = client()
        .fetch_tree(&node.base_url(), &["v1.3".parse().unwrap()])
        .await
        .expect_err("half a document is not a Node");

    assert!(
        matches!(error, NodeApiError::Malformed { .. }),
        "got {error:?}"
    );
}

#[tokio::test]
async fn a_document_of_the_wrong_shape_says_which_collection_failed() {
    let node = FixtureNode::empty().await;
    node.serve_all("v1.3").await;
    node.serve(
        "v1.3",
        "senders",
        ResponseTemplate::new(200).set_body_json(json!([{"id": "not-a-uuid"}])),
    )
    .await;

    let error = client()
        .fetch_tree(&node.base_url(), &["v1.3".parse().unwrap()])
        .await
        .expect_err("a malformed Sender is a malformed tree");

    match error {
        NodeApiError::Malformed { collection, .. } => assert_eq!(collection, "senders"),
        other => panic!("expected a malformed-response error, got {other:?}"),
    }
}

#[tokio::test]
async fn every_failure_names_the_collection_it_came_from() {
    // An operator reading one line on screen needs to know which request went
    // wrong, not merely that something did.
    let node = FixtureNode::empty().await;
    node.serve_all("v1.3").await;
    node.serve("v1.3", "flows", ResponseTemplate::new(503))
        .await;

    let error = client()
        .fetch_tree(&node.base_url(), &["v1.3".parse().unwrap()])
        .await
        .expect_err("503 is a failure");

    assert!(
        error.to_string().contains("flows"),
        "the message does not name the collection: {error}"
    );
}

#[tokio::test]
async fn no_failure_aborts_the_process() {
    // The house rule, stated as a test: every one of these returns.
    let node = FixtureNode::empty().await;
    for response in [
        ResponseTemplate::new(404),
        ResponseTemplate::new(500),
        ResponseTemplate::new(200).set_body_string("not json"),
    ] {
        let one_off = FixtureNode::empty().await;
        one_off.serve("v1.3", "self", response).await;
        let _ = client()
            .fetch_tree(&one_off.base_url(), &["v1.3".parse().unwrap()])
            .await;
    }
    let _ = client()
        .fetch_tree(&node.base_url(), &["v1.3".parse().unwrap()])
        .await;
}
