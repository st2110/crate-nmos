//! A fixture Node: a `wiremock` server that answers the Node API the way real
//! equipment does, built from the published examples so the fixture cannot
//! drift from the contract.

use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{Spec, example};

/// `wiremock` prefers the lower number, and breaks ties by registration order.
const OVERRIDE: u8 = 1;
const DEFAULT: u8 = 5;

/// The six collections a Node API serves.
pub const COLLECTIONS: [&str; 6] = [
    "self",
    "devices",
    "senders",
    "receivers",
    "flows",
    "sources",
];

/// Which published example backs each collection.
fn body(collection: &str) -> Value {
    match collection {
        "self" => example(Spec::Is04, "nodeapi-self-get-200.json"),
        "devices" => example(Spec::Is04, "nodeapi-devices-get-200.json"),
        "senders" => example(Spec::Is04, "nodeapi-senders-get-200.json"),
        "receivers" => example(Spec::Is04, "nodeapi-receivers-get-200.json"),
        "flows" => example(Spec::Is04, "nodeapi-flows-get-200.json"),
        "sources" => example(Spec::Is04, "nodeapi-sources-get-200.json"),
        other => panic!("no example backs the `{other}` collection"),
    }
}

/// A Node API fixture under construction.
pub struct FixtureNode {
    server: MockServer,
}

impl FixtureNode {
    /// A fixture serving the whole Node API at the given version.
    pub async fn serving(version: &str) -> Self {
        let fixture = Self {
            server: MockServer::start().await,
        };
        fixture.serve_all(version).await;
        fixture
    }

    /// A fixture that answers nothing until told to.
    pub async fn empty() -> Self {
        Self {
            server: MockServer::start().await,
        }
    }

    /// Answer one collection at one version with a given response.
    ///
    /// Registered at a higher priority than the defaults, so a test that says
    /// "this collection answers 500" overrides the healthy fixture underneath
    /// it. `wiremock` breaks ties by registration order, which would otherwise
    /// make the override silently do nothing.
    pub async fn serve(&self, version: &str, collection: &str, response: ResponseTemplate) {
        self.mount(version, collection, response, OVERRIDE).await;
    }

    async fn mount(
        &self,
        version: &str,
        collection: &str,
        response: ResponseTemplate,
        priority: u8,
    ) {
        Mock::given(method("GET"))
            .and(path(format!("/x-nmos/node/{version}/{collection}")))
            .respond_with(response)
            .with_priority(priority)
            .mount(&self.server)
            .await;
    }

    /// Answer one collection at one version with the published example.
    pub async fn serve_example(&self, version: &str, collection: &str) {
        self.mount(
            version,
            collection,
            ResponseTemplate::new(200).set_body_json(body(collection)),
            DEFAULT,
        )
        .await;
    }

    /// Answer every collection at one version with the published example.
    pub async fn serve_all(&self, version: &str) {
        for collection in COLLECTIONS {
            self.serve_example(version, collection).await;
        }
    }

    /// Serve a Device list whose control points at this fixture's own
    /// Connection API, which is how the transport pass finds IS-05.
    pub async fn serve_devices_with_connection_control(&self, version: &str, control_urn: &str) {
        let mut devices = body("devices");
        let base = self.base_url();
        for device in devices.as_array_mut().expect("devices is an array") {
            device["controls"] =
                json!([{ "href": format!("{base}/x-nmos/connection/v1.1/"), "type": control_urn }]);
        }
        self.serve(
            version,
            "devices",
            ResponseTemplate::new(200).set_body_json(devices),
        )
        .await;
    }

    /// The base URL callers address this fixture at.
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// The host and port this fixture listens on.
    pub fn address(&self) -> std::net::SocketAddr {
        self.server.address().to_owned()
    }

    /// How many requests the fixture has answered.
    pub async fn requests(&self) -> usize {
        self.server
            .received_requests()
            .await
            .map(|r| r.len())
            .unwrap_or_default()
    }

    /// Every path the fixture has been asked for, in order.
    pub async fn request_paths(&self) -> Vec<String> {
        self.server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .map(|r| r.url.path().to_owned())
            .collect()
    }
}

/// A fixture Connection API, serving IS-05 `active` endpoints.
impl FixtureNode {
    /// Answer one Sender's `active` endpoint.
    pub async fn serve_sender_active(&self, version: &str, id: &str, response: ResponseTemplate) {
        Mock::given(method("GET"))
            .and(path(format!(
                "/x-nmos/connection/{version}/single/senders/{id}/active"
            )))
            .respond_with(response)
            .with_priority(OVERRIDE)
            .mount(&self.server)
            .await;
    }

    /// Answer one Receiver's `active` endpoint.
    pub async fn serve_receiver_active(&self, version: &str, id: &str, response: ResponseTemplate) {
        Mock::given(method("GET"))
            .and(path(format!(
                "/x-nmos/connection/{version}/single/receivers/{id}/active"
            )))
            .respond_with(response)
            .with_priority(OVERRIDE)
            .mount(&self.server)
            .await;
    }

    /// The published `active` response for a Sender.
    pub fn sender_active_example() -> Value {
        example(Spec::Is05, "sender-active-get.json")
    }

    /// The published `active` response for a Receiver.
    pub fn receiver_active_example() -> Value {
        example(Spec::Is05, "receiver-active-get-200.json")
    }
}
