//! The IS-05 Connection API client: reads transport parameters, and nothing else.
//!
//! Two things this client deliberately does not do.
//!
//! It does not report connection state. That already came from the resource
//! tree at six requests per Node, and reading it again here would cost one
//! request per resource for an answer we have — see
//! `DESIGN.md`, "Two-tier reading".
//!
//! It does not write. There is no `staged` PATCH and no activation anywhere in
//! this crate, and `tests/read_only.rs` asserts it, because a controller that
//! could accidentally reconfigure equipment that is on air is not one anybody
//! should run.

use std::fmt;
use std::time::Duration;

use reqwest::StatusCode;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::is05::Param;
use crate::resource::ResourceId;
use crate::version::ApiVersion;

/// The IS-05 versions this client speaks.
pub const SUPPORTED_CONNECTION_VERSIONS: [ApiVersion; 2] =
    [ApiVersion::new(1, 0), ApiVersion::new(1, 1)];

/// The URN prefix a Device uses to advertise its Connection API.
pub const CONNECTION_CONTROL_URN: &str = "urn:x-nmos:control:sr-ctrl";

/// Where a stream actually goes, or comes from.
///
/// This is what makes the connection graph possible on equipment that reports
/// no pairing identifiers: two ends carrying the same address and port are the
/// same stream.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamAddress {
    /// The address, as the device reports it.
    pub address: String,
    /// The port.
    pub port: u16,
}

impl fmt::Display for StreamAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.address, self.port)
    }
}

/// One leg of a Sender's transport. More than one means ST 2022-7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SenderLeg {
    /// Address the stream is sent to. `None` where the device reports `auto`,
    /// or reports nothing — in both cases the controller cannot say where the
    /// stream goes, and must not pretend otherwise.
    pub destination_ip: Option<String>,
    /// Port the stream is sent to, on the same terms.
    pub destination_port: Option<u16>,
    /// Address the stream is sent from.
    pub source_ip: Option<String>,
    /// Whether RTP is enabled on this leg.
    pub rtp_enabled: bool,
}

impl SenderLeg {
    /// Where this leg sends, when that is knowable.
    #[must_use]
    pub fn destination(&self) -> Option<StreamAddress> {
        if !self.rtp_enabled {
            return None;
        }
        Some(StreamAddress {
            address: self.destination_ip.clone()?,
            port: self.destination_port?,
        })
    }
}

/// One leg of a Receiver's transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiverLeg {
    /// Multicast group joined, where the stream is multicast.
    pub multicast_ip: Option<String>,
    /// Address the stream comes from, used for a unicast stream and as the
    /// source filter for a multicast one.
    pub source_ip: Option<String>,
    /// Port the stream arrives on.
    pub destination_port: Option<u16>,
    /// Whether RTP is enabled on this leg.
    pub rtp_enabled: bool,
}

impl ReceiverLeg {
    /// Which stream this leg takes, when that is knowable.
    ///
    /// A multicast group identifies the stream; a unicast stream is identified
    /// by where it comes from. Matching this against a Sender's destination is
    /// how a pairing is recovered when neither end names the other.
    #[must_use]
    pub fn stream(&self) -> Option<StreamAddress> {
        if !self.rtp_enabled {
            return None;
        }
        let address = self
            .multicast_ip
            .clone()
            .or_else(|| self.source_ip.clone())?;
        Some(StreamAddress {
            address,
            port: self.destination_port?,
        })
    }
}

/// What a Sender's Connection API reports about where its stream goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SenderTransport {
    /// The Receiver this Sender was configured against, when it names one.
    pub receiver_id: Option<ResourceId>,
    /// One entry per leg.
    pub legs: Vec<SenderLeg>,
}

/// What a Receiver's Connection API reports about the stream it takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiverTransport {
    /// The Sender this Receiver names, when it names one. Frequently absent —
    /// which is why the legs matter.
    pub sender_id: Option<ResourceId>,
    /// One entry per leg.
    pub legs: Vec<ReceiverLeg>,
    /// Whether the Receiver was connected by transport file rather than by
    /// naming a Sender.
    pub has_transport_file: bool,
}

/// Why a Connection API could not be read.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConnectionApiError {
    /// The connection could not be made.
    #[error("cannot reach the connection api for {resource}: {reason}")]
    Unreachable {
        /// Which resource was being read.
        resource: String,
        /// Why the connection failed.
        reason: String,
    },

    /// The Node accepted the connection but did not answer in time.
    #[error("timed out reading the connection api for {resource}")]
    Timeout {
        /// Which resource was being read.
        resource: String,
    },

    /// The Node answered with an error status.
    #[error("connection api answered {status} for {resource}")]
    Status {
        /// Which resource was being read.
        resource: String,
        /// The status it answered with.
        status: u16,
    },

    /// The Node answered with something that is not a transport.
    #[error("cannot understand the connection api's answer for {resource}: {reason}")]
    Malformed {
        /// Which resource was being read.
        resource: String,
        /// What could not be understood.
        reason: String,
    },

    /// The HTTP client could not be constructed.
    #[error("cannot build an http client: {reason}")]
    Client {
        /// Why.
        reason: String,
    },
}

/// Reads Connection APIs.
#[derive(Debug, Clone)]
pub struct ConnectionApiClient {
    http: reqwest::Client,
}

/// Builds a [`ConnectionApiClient`].
#[derive(Debug, Clone)]
#[must_use]
pub struct ConnectionApiClientBuilder {
    request_timeout: Duration,
    connect_timeout: Duration,
    user_agent: String,
}

impl Default for ConnectionApiClientBuilder {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(2),
            user_agent: concat!("nmos/", env!("CARGO_PKG_VERSION")).to_owned(),
        }
    }
}

impl ConnectionApiClientBuilder {
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
    /// Returns an error if the underlying HTTP client cannot be constructed.
    pub fn build(self) -> Result<ConnectionApiClient, ConnectionApiError> {
        let http = reqwest::Client::builder()
            .timeout(self.request_timeout)
            .connect_timeout(self.connect_timeout)
            .user_agent(self.user_agent)
            .build()
            .map_err(|e| ConnectionApiError::Client {
                reason: e.to_string(),
            })?;
        Ok(ConnectionApiClient { http })
    }
}

impl ConnectionApiClient {
    /// Start building a client.
    pub fn builder() -> ConnectionApiClientBuilder {
        ConnectionApiClientBuilder::default()
    }

    /// Read where one Sender's stream goes.
    ///
    /// # Errors
    ///
    /// Returns the failure, naming the resource it came from.
    pub async fn fetch_sender_transport(
        &self,
        base: &str,
        version: ApiVersion,
        sender: &ResourceId,
    ) -> Result<SenderTransport, ConnectionApiError> {
        let wire: WireSender = self.active(base, version, "senders", sender).await?;
        Ok(SenderTransport {
            receiver_id: wire.receiver_id,
            legs: wire
                .transport_params
                .into_iter()
                .map(SenderLeg::from)
                .collect(),
        })
    }

    /// Read which stream one Receiver takes.
    ///
    /// # Errors
    ///
    /// Returns the failure, naming the resource it came from.
    pub async fn fetch_receiver_transport(
        &self,
        base: &str,
        version: ApiVersion,
        receiver: &ResourceId,
    ) -> Result<ReceiverTransport, ConnectionApiError> {
        let wire: WireReceiver = self.active(base, version, "receivers", receiver).await?;
        Ok(ReceiverTransport {
            sender_id: wire.sender_id,
            legs: wire
                .transport_params
                .into_iter()
                .map(ReceiverLeg::from)
                .collect(),
            has_transport_file: wire.transport_file.is_some_and(|file| file.data.is_some()),
        })
    }

    async fn active<T: DeserializeOwned>(
        &self,
        base: &str,
        version: ApiVersion,
        kind: &str,
        id: &ResourceId,
    ) -> Result<T, ConnectionApiError> {
        let resource = format!("{kind}/{id}");
        let url = format!(
            "{}/x-nmos/connection/{version}/single/{kind}/{id}/active",
            base.trim_end_matches('/')
        );

        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| classify(e, &resource))?;

        let status = response.status();
        if status != StatusCode::OK {
            return Err(ConnectionApiError::Status {
                resource,
                status: status.as_u16(),
            });
        }

        let body = response.text().await.map_err(|e| classify(e, &resource))?;
        serde_json::from_str(&body).map_err(|e| ConnectionApiError::Malformed {
            resource,
            reason: e.to_string(),
        })
    }
}

fn classify(error: reqwest::Error, resource: &str) -> ConnectionApiError {
    let resource = resource.to_owned();
    if error.is_timeout() {
        ConnectionApiError::Timeout { resource }
    } else if error.is_decode() || error.is_body() {
        ConnectionApiError::Malformed {
            resource,
            reason: error.to_string(),
        }
    } else {
        ConnectionApiError::Unreachable {
            resource,
            reason: error.to_string(),
        }
    }
}

// --- the wire ---------------------------------------------------------------

/// A field that arrived as a value, as the string `auto`, or not at all.
///
/// [`Param`] is the same four-state field a patch is built from, read from the
/// other direction. A reader cares about only one distinction — is there a value
/// — so `auto`, `null` and absent all collapse to `None` rather than to a
/// plausible number.
fn known<T>(field: Param<T>) -> Option<T> {
    field.set()
}

/// `master_enable` is deliberately absent from these: connection state comes
/// from the resource tree, and modelling it twice invites the two to disagree.
#[derive(Debug, Deserialize)]
struct WireSender {
    #[serde(default)]
    receiver_id: Option<ResourceId>,
    #[serde(default)]
    transport_params: Vec<WireSenderLeg>,
}

#[derive(Debug, Deserialize)]
struct WireSenderLeg {
    #[serde(default)]
    destination_ip: Param<String>,
    #[serde(default)]
    destination_port: Param<u16>,
    #[serde(default)]
    source_ip: Param<String>,
    #[serde(default)]
    rtp_enabled: bool,
}

impl From<WireSenderLeg> for SenderLeg {
    fn from(leg: WireSenderLeg) -> Self {
        Self {
            destination_ip: known(leg.destination_ip),
            destination_port: known(leg.destination_port),
            source_ip: known(leg.source_ip),
            rtp_enabled: leg.rtp_enabled,
        }
    }
}

#[derive(Debug, Deserialize)]
struct WireReceiver {
    #[serde(default)]
    sender_id: Option<ResourceId>,
    #[serde(default)]
    transport_params: Vec<WireReceiverLeg>,
    #[serde(default)]
    transport_file: Option<WireTransportFile>,
}

#[derive(Debug, Deserialize)]
struct WireTransportFile {
    #[serde(default)]
    data: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WireReceiverLeg {
    #[serde(default)]
    multicast_ip: Param<String>,
    #[serde(default)]
    source_ip: Param<String>,
    #[serde(default)]
    destination_port: Param<u16>,
    #[serde(default)]
    rtp_enabled: bool,
}

impl From<WireReceiverLeg> for ReceiverLeg {
    fn from(leg: WireReceiverLeg) -> Self {
        Self {
            multicast_ip: known(leg.multicast_ip),
            source_ip: known(leg.source_ip),
            destination_port: known(leg.destination_port),
            rtp_enabled: leg.rtp_enabled,
        }
    }
}
