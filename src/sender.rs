//! The Sender resource: an egress from a Device, carrying a Flow onto the network.

use serde::{Deserialize, Serialize};

use crate::connection::Transmission;
use crate::resource::{Capabilities, ResourceCore, ResourceId};

/// An egress from a Device.
///
/// Note what a Sender does *not* tell you. Its subscription says whether it is
/// putting its Flow on the network, and nothing about whether anything is taking
/// it — those are different questions, and only the engine, which sees every
/// Node, can answer the second. See
/// `DESIGN.md`, "Vocabulary".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sender {
    /// The fields every NMOS resource carries.
    #[serde(flatten)]
    pub core: ResourceCore,

    /// Capabilities of this Sender.
    #[serde(default)]
    pub caps: Capabilities,

    /// The Flow currently routed to this Sender, or none when nothing is.
    pub flow_id: Option<ResourceId>,

    /// Transport type URN.
    pub transport: String,

    /// The Device this Sender forms part of.
    pub device_id: ResourceId,

    /// URL of a transport file describing how to connect, when the transport
    /// requires one.
    pub manifest_href: Option<String>,

    /// Names of the Node interfaces this Sender's egress is bound to. More than
    /// one means ST 2022-7 seamless protection.
    #[serde(default)]
    pub interface_bindings: Vec<String>,

    /// How the Sender is currently configured to send.
    pub subscription: SenderSubscription,
}

/// How a Sender is currently configured to send data.
///
/// `active` is IS-04's word. It is deliberately not carried into the domain
/// under that name: [`crate::Transmission`] is what the rest of the project
/// speaks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SenderSubscription {
    /// The Receiver this Sender is configured to send to — only ever set for a
    /// unicast push transport to an NMOS Receiver, and `null` on much
    /// equipment, including the converter this project was built against.
    pub receiver_id: Option<ResourceId>,

    /// IS-04's own flag, kept private on purpose: the only way to ask what a
    /// Sender is doing is [`SenderSubscription::transmission`], which answers
    /// in the domain's words rather than the protocol's.
    #[serde(default)]
    active: bool,
}

impl SenderSubscription {
    /// What the Sender is doing.
    #[must_use]
    pub fn transmission(&self) -> Transmission {
        if self.active {
            Transmission::Transmitting
        } else {
            Transmission::Idle
        }
    }

    /// Build a subscription, for tests and for fabricated fixtures.
    #[must_use]
    pub fn new(transmission: Transmission, receiver_id: Option<ResourceId>) -> Self {
        Self {
            receiver_id,
            active: transmission.is_transmitting(),
        }
    }
}

impl Sender {
    /// What this Sender is doing.
    ///
    /// Available from the resource tree alone, which is why reading state costs
    /// six requests per Node rather than one per resource — see
    /// `DESIGN.md`, "Two-tier reading".
    #[must_use]
    pub fn transmission(&self) -> Transmission {
        self.subscription.transmission()
    }
}
