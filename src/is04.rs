//! AMWA IS-04, Discovery and Registration: the resources a Node has.
//!
//! <https://specs.amwa.tv/is-04/>
//!
//! This module is a map, not a wall. Everything in it is re-exported at the
//! crate root, and the root is where you would normally reach for it —
//! `nmos::Node`, not `nmos::is04::Node`, because people think in Senders and
//! Receivers rather than in specification numbers.
//!
//! What it is for is the other question: *which of these types come from IS-04,
//! and which from somewhere else*. Read down the list and you have the answer.
//!
//! Two things are deliberately absent. [`ApiVersion`](crate::ApiVersion) and
//! [`negotiate`](crate::negotiate) belong to no single specification — both
//! APIs version themselves the same way. [`ParseError`](crate::ParseError) is
//! the crate's own.
//!
//! The documents that *change* these resources are IS-05's, in [`crate::is05`].

/// The fields every resource carries, and the scalars inside them.
pub use crate::resource::{Capabilities, ResourceCore, ResourceId, Tags, Version};

/// What a Sender and a Receiver are each doing, derived from `subscription`.
///
/// IS-04 writes `active` at both ends. This crate does not: see `DESIGN.md`,
/// "Vocabulary", for why one word covering two different facts is worth
/// replacing with two.
pub use crate::connection::{Reception, Transmission};

/// How media is described wherever IS-04 describes it.
pub use crate::media::{Format, MediaType, Rate};

/// The Node itself, and how it says it can be reached.
pub use crate::node::{
    ApiEndpoint, AttachedNetworkDevice, Clock, Interface, Node, NodeApi, Protocol, Service,
};

/// A Device, and the controls it advertises — which is how a controller finds
/// the Connection API.
pub use crate::device::{Control, Device};

/// A Sender, and the subscription saying whether it is transmitting.
pub use crate::sender::{Sender, SenderSubscription};

/// A Receiver, what it will accept, and the subscription saying whether it is
/// taking anything.
pub use crate::receiver::{MediaCaps, Receiver, ReceiverCaps, ReceiverSubscription};

/// A Flow: what a Sender actually carries.
pub use crate::flow::{
    Component, ComponentName, DidSdid, Flow, FlowCore, InterlaceMode, VideoCore,
};

/// A Source: where a Flow originates.
pub use crate::source::{Channel, Source, SourceCore};
