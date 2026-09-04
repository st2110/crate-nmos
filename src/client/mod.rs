//! Talking to a Node over HTTP: the IS-04 Node API and the IS-05 Connection
//! API.
//!
//! Everything here reads. Changing a device is a separate capability and will
//! arrive behind its own feature — see `DESIGN.md`, "What is missing".

mod connection_api;
mod node_api;

pub use connection_api::{
    CONNECTION_CONTROL_URN, ConnectionApiClient, ConnectionApiClientBuilder, ConnectionApiError,
    ReceiverLeg, ReceiverTransport, SUPPORTED_CONNECTION_VERSIONS, SenderLeg, SenderTransport,
    StreamAddress,
};
pub use node_api::{
    CollectionData, NodeApiClient, NodeApiClientBuilder, NodeApiError, NodeCollection,
    ResourceTree, SUPPORTED_VERSIONS,
};
