//! The Receiver resource: an ingress to a Device, consuming a stream.

use serde::{Deserialize, Serialize};

use crate::connection::Reception;
use crate::media::MediaType;
use crate::resource::{ResourceCore, ResourceId};

/// An ingress to a Device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receiver {
    /// The fields every NMOS resource carries.
    #[serde(flatten)]
    pub core: ResourceCore,

    /// The Device this Receiver forms part of.
    pub device_id: ResourceId,

    /// Transport type URN the Receiver accepts.
    pub transport: String,

    /// Names of the Node interfaces this Receiver's ingress is bound to.
    #[serde(default)]
    pub interface_bindings: Vec<String>,

    /// How the Receiver is currently configured to receive.
    pub subscription: ReceiverSubscription,

    /// What the Receiver accepts, and in which format.
    #[serde(flatten)]
    pub caps: ReceiverCaps,
}

/// How a Receiver is currently configured to receive data.
///
/// As with a Sender, `active` is IS-04's word and stops at this boundary:
/// [`crate::Reception`] is the domain's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiverSubscription {
    /// The Sender this Receiver is taking from, when the Node reports one.
    /// Frequently `null` even on a Receiver that is taking a stream, which is
    /// why the pairing also has to be recovered from transport parameters.
    pub sender_id: Option<ResourceId>,

    /// IS-04's own flag, kept private on purpose; ask
    /// [`ReceiverSubscription::reception`] instead.
    #[serde(default)]
    active: bool,
}

impl ReceiverSubscription {
    /// What the Receiver is doing.
    #[must_use]
    pub fn reception(&self) -> Reception {
        if self.active {
            Reception::Subscribed
        } else {
            Reception::Unsubscribed
        }
    }

    /// Build a subscription, for tests and for fabricated fixtures.
    #[must_use]
    pub fn new(reception: Reception, sender_id: Option<ResourceId>) -> Self {
        Self {
            sender_id,
            active: reception.is_subscribed(),
        }
    }
}

impl Receiver {
    /// What this Receiver is doing.
    #[must_use]
    pub fn reception(&self) -> Reception {
        self.subscription.reception()
    }
}

/// What a Receiver accepts, keyed by the format it declares.
///
/// The specification models this as four separate resource schemas, one per
/// format; here it is one enum, which is the same information in the shape Rust
/// gives a reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "format")]
pub enum ReceiverCaps {
    #[serde(rename = "urn:x-nmos:format:video")]
    Video {
        /// Capabilities of a video Receiver.
        caps: MediaCaps,
    },

    #[serde(rename = "urn:x-nmos:format:audio")]
    Audio {
        /// Capabilities of an audio Receiver.
        caps: MediaCaps,
    },

    #[serde(rename = "urn:x-nmos:format:data")]
    Data {
        /// Capabilities of a data Receiver.
        caps: MediaCaps,
    },

    #[serde(rename = "urn:x-nmos:format:mux")]
    Mux {
        /// Capabilities of a mux Receiver.
        caps: MediaCaps,
    },
}

impl ReceiverCaps {
    /// The media types this Receiver accepts.
    ///
    /// These need not agree with what any Sender emits — the bench converter
    /// advertises `audio/L24` on Receivers whose neighbouring Senders emit
    /// `audio/L16` — so they are reported, never reconciled.
    #[must_use]
    pub fn media_types(&self) -> &[MediaType] {
        match self {
            ReceiverCaps::Video { caps }
            | ReceiverCaps::Audio { caps }
            | ReceiverCaps::Data { caps }
            | ReceiverCaps::Mux { caps } => &caps.media_types,
        }
    }

    /// The format this Receiver accepts.
    #[must_use]
    pub fn format(&self) -> crate::Format {
        match self {
            ReceiverCaps::Video { .. } => crate::Format::Video,
            ReceiverCaps::Audio { .. } => crate::Format::Audio,
            ReceiverCaps::Data { .. } => crate::Format::Data,
            ReceiverCaps::Mux { .. } => crate::Format::Mux,
        }
    }
}

/// The media a Receiver will accept.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MediaCaps {
    /// Subclassifications of the format accepted.
    #[serde(default)]
    pub media_types: Vec<MediaType>,

    /// IS-07 event types accepted, on a data Receiver that carries them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_types: Vec<String>,
}
