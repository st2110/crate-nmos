//! AMWA NMOS for SMPTE ST 2110 networks: the IS-04 resource model, IS-05
//! connection management, and clients for both.
//!
//! This crate knows the protocol and nothing else. It holds no state, performs
//! no discovery of its own, applies no policy, and has no opinion about how
//! anything is displayed. Two first-party consumers pull on it from opposite
//! ends — a controller that reads Nodes it did not build, and a Node that
//! serves its own resources — and `DESIGN.md` records which shape serves both.
//!
//! The types are written by hand rather than generated, and are held to the
//! published AMWA JSON Schemas by the test suite in both directions: what
//! parses must serialise back into something the schema accepts.
//!
//! # Where things come from
//!
//! The resources — [`Node`], [`Device`], [`Sender`], [`Receiver`], [`Source`],
//! [`Flow`] and what they are made of — are here at the root, because they are
//! the vocabulary and nobody wants to write a specification number to name a
//! Sender. [`is04`] lists them anyway, so that "which of these is IS-04" has an
//! answer you can read.
//!
//! [`is05`] holds the connection documents — `staged`, `active`, constraints
//! and the patches between them. Those are one API's request and response
//! bodies rather than things a Node has, so they stay in their module.
//!
//! # Features
//!
//! | feature | default | what it adds |
//! |---|---|---|
//! | `client` | yes | HTTP clients for the Node API and the Connection API |
//! | `uuid` | no | resource identifiers as `Uuid`, for the generating side |
//!
//! A consumer that wants the types and no I/O at all takes the crate with
//! `default-features = false`.

// This crate drives equipment that is on air, so it returns errors rather than
// aborting (AGENTS.md). Tests are the one place where a panic is the clearest
// way to assert.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

mod connection;
mod device;
mod error;
mod flow;
pub mod is04;
pub mod is05;
mod media;
mod node;
mod receiver;
mod resource;
mod sender;
mod source;
mod version;

#[cfg(feature = "client")]
mod client;

pub use connection::{Reception, Transmission};
pub use device::{Control, Device};
pub use error::ParseError;
pub use flow::{Component, ComponentName, DidSdid, Flow, FlowCore, InterlaceMode, VideoCore};
pub use media::{Format, MediaType, Rate};
pub use node::{
    ApiEndpoint, AttachedNetworkDevice, Clock, Interface, Node, NodeApi, Protocol, Service,
};
pub use receiver::{MediaCaps, Receiver, ReceiverCaps, ReceiverSubscription};
pub use resource::{Capabilities, ResourceCore, ResourceId, Tags, Version};
pub use sender::{Sender, SenderSubscription};
pub use source::{Channel, Source, SourceCore};
pub use version::{ApiVersion, negotiate};

#[cfg(feature = "client")]
pub use client::{
    CONNECTION_CONTROL_URN, CollectionData, ConnectionApiClient, ConnectionApiClientBuilder,
    ConnectionApiError, NodeApiClient, NodeApiClientBuilder, NodeApiError, NodeCollection,
    ReceiverLeg, ReceiverTransport, ResourceTree, SUPPORTED_CONNECTION_VERSIONS,
    SUPPORTED_VERSIONS, SenderLeg, SenderTransport, StreamAddress,
};
