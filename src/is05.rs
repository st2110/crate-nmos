//! AMWA IS-05, Device Connection Management: `staged`, `active`, and the patches
//! that move one into the other.
//!
//! <https://specs.amwa.tv/is-05/>
//!
//! Both ends of the protocol read these. A controller composes a patch; a Node
//! parses it, decides whether it can be honoured, and answers with the resource
//! it now holds. Neither side owns the shapes, which is why they live here and
//! not in either application.
//!
//! Unlike the IS-04 resources in [`crate::is04`], these are not re-exported at
//! the crate root: they describe one API's request and response bodies rather
//! than the things a Node has, and only a consumer doing connection management
//! ever names them.

use std::collections::BTreeMap;
use std::net::IpAddr;

use serde::de::{DeserializeOwned, Error as _};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::resource::{ResourceId, Version};

/// The only transport file type RTP has.
pub const TRANSPORT_FILE_TYPE: &str = "application/sdp";

/// The literal a controller sends to say "you decide".
const AUTO: &str = "auto";

/// A field in a patch: set, `"auto"`, `null`, or not mentioned at all.
///
/// Four states rather than `Option<Option<T>>`, because all four mean different
/// things to whoever applies the patch:
///
/// * [`Param::Absent`] — leave this alone,
/// * [`Param::Null`] — clear it,
/// * [`Param::Auto`] — decide it yourself,
/// * [`Param::Set`] — use this.
///
/// A reader of `active` documents only ever meets the last two, and can ignore
/// the distinction. A Node applying a patch cannot: "leave it" and "clear it"
/// are opposite instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Param<T> {
    /// The key was not in the document.
    #[default]
    Absent,
    /// The key was present and `null`.
    Null,
    /// The key was present and the string `auto`.
    Auto,
    /// The key carried a value.
    Set(T),
}

impl<T> Param<T> {
    /// What to do with the field: `None` leaves it as it was, `Some(next)`
    /// replaces it.
    ///
    /// `auto` collapses into "no value imposed" rather than getting a state of
    /// its own here: what a Node picks for an automatic parameter is its own
    /// business, and the caller has already been told it may choose.
    #[must_use]
    pub fn value(self) -> Option<Option<T>> {
        match self {
            Self::Absent => None,
            Self::Null | Self::Auto => Some(None),
            Self::Set(value) => Some(Some(value)),
        }
    }

    /// Whether the key was missing from the document.
    ///
    /// Used as `skip_serializing_if`, so that composing a patch leaves out the
    /// fields it does not speak about instead of writing `null` over them.
    #[must_use]
    pub const fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    /// The value, if one was given.
    #[must_use]
    pub fn set(self) -> Option<T> {
        match self {
            Self::Set(value) => Some(value),
            _ => None,
        }
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Param<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Value::deserialize(deserializer)? {
            Value::Null => Ok(Self::Null),
            Value::String(text) if text == AUTO => Ok(Self::Auto),
            other => T::deserialize(other)
                .map(Self::Set)
                .map_err(D::Error::custom),
        }
    }
}

impl<T: Serialize> Serialize for Param<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            // `Absent` cannot be written as a key that is not there, so a struct
            // holding one skips the field entirely. Serialised on its own it is
            // indistinguishable from `Null`, which is the honest answer: outside
            // a document, "missing" has no representation.
            Self::Absent | Self::Null => serializer.serialize_none(),
            Self::Auto => serializer.serialize_str(AUTO),
            Self::Set(value) => value.serialize(serializer),
        }
    }
}

/// When a staged connection should take effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ActivationMode {
    /// As soon as the request is answered.
    ActivateImmediate,
    /// At a stated time on the shared timescale.
    ActivateScheduledAbsolute,
    /// After a stated interval.
    ActivateScheduledRelative,
}

/// The activation block of a `staged` or `active` resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Activation {
    /// What was asked for. Absent once it has happened.
    pub mode: Option<ActivationMode>,
    /// When it was asked to happen.
    pub requested_time: Option<Version>,
    /// When it did.
    pub activation_time: Option<Version>,
}

impl Activation {
    /// An immediate activation that has just happened.
    #[must_use]
    pub const fn immediate(at: Version) -> Self {
        Self {
            mode: Some(ActivationMode::ActivateImmediate),
            requested_time: None,
            activation_time: Some(at),
        }
    }

    /// The trace an activation leaves behind: the time stays, the mode goes.
    ///
    /// A resource that keeps answering `activate_immediate` is claiming a
    /// pending activation it does not have.
    #[must_use]
    pub const fn settled(self) -> Self {
        Self {
            mode: None,
            requested_time: None,
            activation_time: self.activation_time,
        }
    }
}

/// The transport file of a connection, which for RTP is an SDP document.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TransportFile {
    /// The document itself.
    pub data: Option<String>,
    /// Its media type.
    #[serde(rename = "type")]
    pub file_type: Option<String>,
}

impl TransportFile {
    /// An SDP transport file, or an empty one.
    ///
    /// The type follows the data: announcing `application/sdp` over a document
    /// that is not there describes nothing.
    #[must_use]
    pub fn sdp(data: Option<String>) -> Self {
        Self {
            file_type: data.as_ref().map(|_| TRANSPORT_FILE_TYPE.to_owned()),
            data,
        }
    }
}

/// What a Node will accept in one transport parameter.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Constraint {
    /// Smallest acceptable value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,
    /// Largest acceptable value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,
    /// The complete set of acceptable values.
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub allowed: Option<Vec<Value>>,
}

impl Constraint {
    /// A parameter that may only take the values given.
    ///
    /// An interface expressed as a one-element enumeration is not a formality:
    /// it is how a controller learns which wire a stream can arrive on.
    #[must_use]
    pub fn one_of<I, V>(values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        Self {
            allowed: Some(values.into_iter().map(Into::into).collect()),
            ..Self::default()
        }
    }

    /// A parameter that is a usable IP port.
    #[must_use]
    pub fn port() -> Self {
        Self::between(1, i64::from(u16::MAX))
    }

    /// A parameter bounded at both ends.
    #[must_use]
    pub const fn between(minimum: i64, maximum: i64) -> Self {
        Self {
            minimum: Some(minimum),
            maximum: Some(maximum),
            allowed: None,
        }
    }
}

/// The constraints of one leg, by parameter name.
pub type LegConstraints = BTreeMap<String, Constraint>;

/// A patch against a Receiver's `staged` resource.
///
/// Generic in the transport family, because the documents do not say which one
/// they belong to: whether a leg is RTP or MQTT is settled by the Receiver's
/// `transport` field, which lives in IS-04. So the caller names the family — a
/// Node knows its own, and a controller has just read it — and gets a type that
/// refuses anything else. [`UnknownParams`] is the way out when the transport is
/// one this crate does not model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReceiverStagedPatch<P = ReceiverRtpParams> {
    /// The Sender being taken, or `null` to take nobody.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub sender_id: Param<ResourceId>,
    /// Whether the Receiver should be subscribed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_enable: Option<bool>,
    /// When to apply this.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation: Option<ActivationPatch>,
    /// The SDP describing what to receive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_file: Option<TransportFile>,
    /// One entry per leg. A redundant Receiver has two.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_params: Option<Vec<P>>,
}

/// A patch against a Sender's `staged` resource. Generic in the transport
/// family, for the reasons given on [`ReceiverStagedPatch`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SenderStagedPatch<P = SenderRtpParams> {
    /// The Receiver being fed, or `null` for none.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub receiver_id: Param<ResourceId>,
    /// Whether the Sender should transmit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_enable: Option<bool>,
    /// When to apply this.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation: Option<ActivationPatch>,
    /// One entry per leg.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_params: Option<Vec<P>>,
}

/// The activation block of a patch.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ActivationPatch {
    /// What is being asked for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ActivationMode>,
    /// When, for the scheduled modes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_time: Option<Version>,
    /// Read-only: the Node sets this. Accepted because controllers hand back
    /// the whole resource, and ignored because it is not theirs to set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_time: Option<Version>,
}

/// One leg of a transport this crate does not model.
///
/// Every key is kept as it arrived and handed back unchanged. A controller that
/// meets equipment speaking MXL, or anything else added to NMOS after this was
/// written, can still read it and give it back — refusing would make this crate
/// the reason an operator cannot see their device.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UnknownParams(BTreeMap<String, Value>);

impl UnknownParams {
    /// One parameter, as it arrived.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    /// Whether the leg says anything at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Every parameter, in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.0.iter()
    }
}

/// One leg of a Receiver reached over a websocket.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReceiverWebsocketParams {
    /// Where to connect.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub connection_uri: Param<String>,
    /// How to authorize, or `false` for not at all. The schema allows a string
    /// or a boolean, so this holds either rather than half of what is legal.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub connection_authorization: Param<Value>,
}

/// One leg of a Sender reached over a websocket.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SenderWebsocketParams {
    /// Where to connect.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub connection_uri: Param<String>,
    /// How to authorize, or `false` for not at all.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub connection_authorization: Param<Value>,
}

/// One leg of a Receiver subscribed to an MQTT broker.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReceiverMqttParams {
    /// The broker's host.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub source_host: Param<String>,
    /// The broker's port.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub source_port: Param<u16>,
    /// Which MQTT flavour the broker speaks.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub broker_protocol: Param<String>,
    /// How to authorize with the broker, or `false` for not at all.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub broker_authorization: Param<Value>,
    /// The topic carrying the media.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub broker_topic: Param<String>,
    /// The topic carrying connection status.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub connection_status_broker_topic: Param<String>,
}

/// One leg of a Sender publishing to an MQTT broker.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SenderMqttParams {
    /// The broker's host.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub destination_host: Param<String>,
    /// The broker's port.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub destination_port: Param<u16>,
    /// Which MQTT flavour the broker speaks.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub broker_protocol: Param<String>,
    /// How to authorize with the broker, or `false` for not at all.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub broker_authorization: Param<Value>,
    /// The topic carrying the media.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub broker_topic: Param<String>,
    /// The topic carrying connection status.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub connection_status_broker_topic: Param<String>,
}

/// A patch against one leg of a Receiver's RTP transport parameters.
///
/// Every field is a [`Param`], so "not mentioned" stays distinguishable from
/// "cleared" all the way through.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
// Deliberately not `#[non_exhaustive]`. The fields are AMWA's, not ours, and a
// Node has to build one of these to answer a controller at all; a struct that
// can only be reached through `Default` serves the reading half of this crate's
// audience and refuses the writing half.
#[serde(default, deny_unknown_fields)]
pub struct ReceiverRtpParams {
    /// Source to filter on, for source-specific multicast.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub source_ip: Param<IpAddr>,
    /// Multicast group to join.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub multicast_ip: Param<IpAddr>,
    /// Which of the Node's interfaces receives.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub interface_ip: Param<IpAddr>,
    /// Port to receive on.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub destination_port: Param<u16>,
    /// Whether RTP is being received at all.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub rtp_enabled: Param<bool>,
    /// Whether forward error correction is in use.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub fec_enabled: Param<bool>,
    /// Where the FEC streams arrive.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub fec_destination_ip: Param<IpAddr>,
    /// One- or two-dimensional FEC.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub fec_mode: Param<String>,
    /// Port of the first FEC stream.
    #[serde(
        rename = "fec1D_destination_port",
        skip_serializing_if = "Param::is_absent"
    )]
    pub fec1d_destination_port: Param<u16>,
    /// Port of the second FEC stream.
    #[serde(
        rename = "fec2D_destination_port",
        skip_serializing_if = "Param::is_absent"
    )]
    pub fec2d_destination_port: Param<u16>,
    /// Whether RTCP is in use.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub rtcp_enabled: Param<bool>,
    /// Where RTCP arrives.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub rtcp_destination_ip: Param<IpAddr>,
    /// The RTCP port.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub rtcp_destination_port: Param<u16>,
}

/// A patch against one leg of a Sender's RTP transport parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
// Deliberately not `#[non_exhaustive]`. The fields are AMWA's, not ours, and a
// Node has to build one of these to answer a controller at all; a struct that
// can only be reached through `Default` serves the reading half of this crate's
// audience and refuses the writing half.
#[serde(default, deny_unknown_fields)]
pub struct SenderRtpParams {
    /// Which of the Node's interfaces transmits.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub source_ip: Param<IpAddr>,
    /// Where the stream is sent.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub destination_ip: Param<IpAddr>,
    /// Port transmitted from.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub source_port: Param<u16>,
    /// Port sent to.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub destination_port: Param<u16>,
    /// Whether RTP is being sent at all.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub rtp_enabled: Param<bool>,
    /// Whether forward error correction is in use.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub fec_enabled: Param<bool>,
    /// Where the FEC streams are sent.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub fec_destination_ip: Param<IpAddr>,
    /// Which FEC scheme.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub fec_type: Param<String>,
    /// One- or two-dimensional FEC.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub fec_mode: Param<String>,
    /// FEC matrix width.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub fec_block_width: Param<u32>,
    /// FEC matrix height.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub fec_block_height: Param<u32>,
    /// Destination port of the first FEC stream.
    #[serde(
        rename = "fec1D_destination_port",
        skip_serializing_if = "Param::is_absent"
    )]
    pub fec1d_destination_port: Param<u16>,
    /// Destination port of the second FEC stream.
    #[serde(
        rename = "fec2D_destination_port",
        skip_serializing_if = "Param::is_absent"
    )]
    pub fec2d_destination_port: Param<u16>,
    /// Source port of the first FEC stream.
    #[serde(rename = "fec1D_source_port", skip_serializing_if = "Param::is_absent")]
    pub fec1d_source_port: Param<u16>,
    /// Source port of the second FEC stream.
    #[serde(rename = "fec2D_source_port", skip_serializing_if = "Param::is_absent")]
    pub fec2d_source_port: Param<u16>,
    /// Whether RTCP is in use.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub rtcp_enabled: Param<bool>,
    /// Where RTCP is sent.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub rtcp_destination_ip: Param<IpAddr>,
    /// The RTCP destination port.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub rtcp_destination_port: Param<u16>,
    /// The RTCP source port.
    #[serde(skip_serializing_if = "Param::is_absent")]
    pub rtcp_source_port: Param<u16>,
}
