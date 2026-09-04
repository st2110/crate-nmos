//! The IS-04 Node API client: reads a Node's resource tree, and nothing else.
//!
//! Read-only by contract. There is no method here that changes a device, and
//! `tests/read_only.rs` asserts as much — a controller that could accidentally
//! reconfigure equipment that is on air is not a controller anyone should run.

use std::time::Duration;

use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::device::Device;
use crate::flow::Flow;
use crate::node::Node;
use crate::receiver::Receiver;
use crate::sender::Sender;
use crate::source::Source;
use crate::version::{ApiVersion, negotiate};

/// The IS-04 versions this client speaks, highest last.
///
/// Only `v1.2` and `v1.3` — the fields this project reads are unchanged across
/// them, so one set of types serves both and negotiation is a URL prefix rather
/// than a second model. Supporting only `v1.3` would be simpler and would make
/// the tool useless on the many deployed devices that speak `v1.2`.
pub const SUPPORTED_VERSIONS: [ApiVersion; 2] = [ApiVersion::new(1, 2), ApiVersion::new(1, 3)];

/// One of the six collections a Node API serves.
///
/// The paths and the `ver_*` keys are both IS-04's, so they live together here
/// rather than being split between the protocol and whatever reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeCollection {
    /// The Node's own record.
    Node,
    /// The Devices the Node hosts.
    Devices,
    /// The Senders those Devices expose.
    Senders,
    /// The Receivers those Devices expose.
    Receivers,
    /// The Flows the Senders carry.
    Flows,
    /// The Sources those Flows originate from.
    Sources,
}

impl NodeCollection {
    /// Every collection, in the order a Node's tree is read.
    pub const ALL: [NodeCollection; 6] = [
        NodeCollection::Node,
        NodeCollection::Devices,
        NodeCollection::Senders,
        NodeCollection::Receivers,
        NodeCollection::Flows,
        NodeCollection::Sources,
    ];

    /// The path segment this collection is read from.
    #[must_use]
    pub fn path(self) -> &'static str {
        match self {
            NodeCollection::Node => "self",
            NodeCollection::Devices => "devices",
            NodeCollection::Senders => "senders",
            NodeCollection::Receivers => "receivers",
            NodeCollection::Flows => "flows",
            NodeCollection::Sources => "sources",
        }
    }

    /// The DNS-SD TXT record key this collection's version counter is
    /// published under.
    #[must_use]
    pub fn counter_key(self) -> &'static str {
        match self {
            NodeCollection::Node => "ver_slf",
            NodeCollection::Devices => "ver_dvc",
            NodeCollection::Senders => "ver_snd",
            NodeCollection::Receivers => "ver_rcv",
            NodeCollection::Flows => "ver_flw",
            NodeCollection::Sources => "ver_src",
        }
    }
}

impl std::fmt::Display for NodeCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.path())
    }
}

/// What one collection of a Node's tree contains.
#[derive(Debug, Clone, PartialEq)]
pub enum CollectionData {
    /// The Node's own record.
    Node(Box<Node>),
    /// The Devices the Node hosts.
    Devices(Vec<Device>),
    /// The Senders those Devices expose.
    Senders(Vec<Sender>),
    /// The Receivers those Devices expose.
    Receivers(Vec<Receiver>),
    /// The Flows the Senders carry.
    Flows(Vec<Flow>),
    /// The Sources those Flows originate from.
    Sources(Vec<Source>),
}

impl CollectionData {
    /// Which collection this is.
    #[must_use]
    pub fn collection(&self) -> NodeCollection {
        match self {
            CollectionData::Node(_) => NodeCollection::Node,
            CollectionData::Devices(_) => NodeCollection::Devices,
            CollectionData::Senders(_) => NodeCollection::Senders,
            CollectionData::Receivers(_) => NodeCollection::Receivers,
            CollectionData::Flows(_) => NodeCollection::Flows,
            CollectionData::Sources(_) => NodeCollection::Sources,
        }
    }
}

/// Everything a Node says about itself, read in one pass.
///
/// This is the whole of the cheap tier: six requests, and every Sender and
/// Receiver already carries its connection state. See
/// `DESIGN.md`, "Two-tier reading".
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceTree {
    /// The version the tree was read at.
    pub version: ApiVersion,
    /// The Node's own record.
    pub node: Node,
    /// The Devices the Node hosts.
    pub devices: Vec<Device>,
    /// The Senders those Devices expose.
    pub senders: Vec<Sender>,
    /// The Receivers those Devices expose.
    pub receivers: Vec<Receiver>,
    /// The Flows the Senders carry.
    pub flows: Vec<Flow>,
    /// The Sources those Flows originate from.
    pub sources: Vec<Source>,
}

/// Why a Node could not be read.
///
/// Every variant names the collection it came from: an operator reading one
/// line against one Node needs to know which request went wrong.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum NodeApiError {
    /// The Node offers no version this client speaks.
    #[error("node offers only {} (this controller speaks {})", offered.join(", "), supported.join(", "))]
    UnsupportedVersion {
        /// The versions the Node advertised.
        offered: Vec<String>,
        /// The versions this client speaks.
        supported: Vec<String>,
    },

    /// The Node's address could not be turned into a URL.
    #[error("cannot address the node at {base}: {reason}")]
    BadAddress {
        /// The base URL that was tried.
        base: String,
        /// Why it could not be used.
        reason: String,
    },

    /// The connection could not be made.
    #[error("cannot reach the node for {collection}: {reason}")]
    Unreachable {
        /// Which collection was being read.
        collection: &'static str,
        /// Why the connection failed.
        reason: String,
    },

    /// The Node accepted the connection but did not answer in time.
    #[error("timed out reading {collection}")]
    Timeout {
        /// Which collection was being read.
        collection: &'static str,
    },

    /// The Node answered with an error status.
    #[error("node answered {status} for {collection}")]
    Status {
        /// Which collection was being read.
        collection: &'static str,
        /// The status it answered with.
        status: u16,
    },

    /// The Node answered with something that is not the resource it should be.
    #[error("cannot understand the node's {collection}: {reason}")]
    Malformed {
        /// Which collection was being read.
        collection: &'static str,
        /// What could not be understood.
        reason: String,
    },
}

/// Reads Node APIs.
///
/// Cheap to clone: the underlying HTTP client pools connections, so one client
/// serves every Node on the network.
#[derive(Debug, Clone)]
pub struct NodeApiClient {
    http: reqwest::Client,
}

/// Builds a [`NodeApiClient`].
#[derive(Debug, Clone)]
#[must_use]
pub struct NodeApiClientBuilder {
    request_timeout: Duration,
    connect_timeout: Duration,
    user_agent: String,
}

impl Default for NodeApiClientBuilder {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(2),
            user_agent: concat!("nmos/", env!("CARGO_PKG_VERSION")).to_owned(),
        }
    }
}

impl NodeApiClientBuilder {
    /// How long one request may take in total.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// How long establishing a connection may take.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// What to identify this controller as.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Build the client.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be constructed,
    /// which in practice means the platform's TLS backend is unavailable.
    pub fn build(self) -> Result<NodeApiClient, NodeApiError> {
        let http = reqwest::Client::builder()
            .timeout(self.request_timeout)
            .connect_timeout(self.connect_timeout)
            .user_agent(self.user_agent)
            .build()
            .map_err(|e| NodeApiError::BadAddress {
                base: "<http client>".to_owned(),
                reason: e.to_string(),
            })?;
        Ok(NodeApiClient { http })
    }
}

impl NodeApiClient {
    /// Start building a client.
    pub fn builder() -> NodeApiClientBuilder {
        NodeApiClientBuilder::default()
    }

    /// Read a Node's whole resource tree.
    ///
    /// `offered` is what the Node advertised in its `api_ver` TXT record. The
    /// highest version both ends understand is used; requests are made in
    /// sequence, because small equipment answers one at a time.
    ///
    /// # Errors
    ///
    /// Returns [`NodeApiError::UnsupportedVersion`] when nothing overlaps, and
    /// otherwise the first failure encountered, naming the collection it came
    /// from.
    pub async fn fetch_tree(
        &self,
        base: &str,
        offered: &[ApiVersion],
    ) -> Result<ResourceTree, NodeApiError> {
        let version = negotiate(offered, &SUPPORTED_VERSIONS).ok_or_else(|| {
            NodeApiError::UnsupportedVersion {
                offered: offered.iter().map(ApiVersion::to_string).collect(),
                supported: SUPPORTED_VERSIONS
                    .iter()
                    .map(ApiVersion::to_string)
                    .collect(),
            }
        })?;

        let prefix = format!("{}/x-nmos/node/{version}", base.trim_end_matches('/'));

        Ok(ResourceTree {
            version,
            node: self.collection(&prefix, "self").await?,
            devices: self.collection(&prefix, "devices").await?,
            senders: self.collection(&prefix, "senders").await?,
            receivers: self.collection(&prefix, "receivers").await?,
            flows: self.collection(&prefix, "flows").await?,
            sources: self.collection(&prefix, "sources").await?,
        })
    }

    /// Read one collection of a Node's tree.
    ///
    /// The whole tree costs six requests; one collection costs one. That is
    /// what makes counter-driven refresh worth having — a Node whose Receivers
    /// changed does not need its Flows read again.
    ///
    /// # Errors
    ///
    /// Returns [`NodeApiError::UnsupportedVersion`] when nothing overlaps, and
    /// otherwise the failure, naming the collection.
    pub async fn fetch_collection(
        &self,
        base: &str,
        offered: &[ApiVersion],
        collection: NodeCollection,
    ) -> Result<CollectionData, NodeApiError> {
        let version = self.negotiated(offered)?;
        let prefix = format!("{}/x-nmos/node/{version}", base.trim_end_matches('/'));
        let path = collection.path();

        Ok(match collection {
            NodeCollection::Node => {
                CollectionData::Node(Box::new(self.collection(&prefix, path).await?))
            }
            NodeCollection::Devices => {
                CollectionData::Devices(self.collection(&prefix, path).await?)
            }
            NodeCollection::Senders => {
                CollectionData::Senders(self.collection(&prefix, path).await?)
            }
            NodeCollection::Receivers => {
                CollectionData::Receivers(self.collection(&prefix, path).await?)
            }
            NodeCollection::Flows => CollectionData::Flows(self.collection(&prefix, path).await?),
            NodeCollection::Sources => {
                CollectionData::Sources(self.collection(&prefix, path).await?)
            }
        })
    }

    /// The highest version this client and the Node both understand.
    fn negotiated(&self, offered: &[ApiVersion]) -> Result<ApiVersion, NodeApiError> {
        negotiate(offered, &SUPPORTED_VERSIONS).ok_or_else(|| NodeApiError::UnsupportedVersion {
            offered: offered.iter().map(ApiVersion::to_string).collect(),
            supported: SUPPORTED_VERSIONS
                .iter()
                .map(ApiVersion::to_string)
                .collect(),
        })
    }

    /// Read one collection, turning every way it can go wrong into a value that
    /// names the collection.
    async fn collection<T: DeserializeOwned>(
        &self,
        prefix: &str,
        collection: &'static str,
    ) -> Result<T, NodeApiError> {
        let response = self
            .http
            .get(format!("{prefix}/{collection}"))
            .send()
            .await
            .map_err(|e| classify(e, collection))?;

        let status = response.status();
        if status != StatusCode::OK {
            return Err(NodeApiError::Status {
                collection,
                status: status.as_u16(),
            });
        }

        // Read as text first: equipment answers with a login page often enough
        // that "expected JSON, got HTML" has to be a message and not a puzzle.
        let body = response.text().await.map_err(|e| classify(e, collection))?;
        serde_json::from_str(&body).map_err(|e| NodeApiError::Malformed {
            collection,
            reason: describe(&body, &e),
        })
    }
}

/// Turn a transport failure into the error that names what an operator should
/// look at.
fn classify(error: reqwest::Error, collection: &'static str) -> NodeApiError {
    if error.is_timeout() {
        NodeApiError::Timeout { collection }
    } else if error.is_decode() || error.is_body() {
        NodeApiError::Malformed {
            collection,
            reason: error.to_string(),
        }
    } else {
        NodeApiError::Unreachable {
            collection,
            reason: error.to_string(),
        }
    }
}

/// Describe a parse failure in terms of what actually arrived.
fn describe(body: &str, error: &serde_json::Error) -> String {
    let looks_like_markup = body.trim_start().starts_with('<');
    if looks_like_markup {
        format!("expected JSON, got what looks like a web page ({error})")
    } else {
        error.to_string()
    }
}
