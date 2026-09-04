//! The Node resource: one logical host on the network, as it announces itself.

use serde::{Deserialize, Serialize};

use crate::resource::{Capabilities, ResourceCore};

/// One logical host, holding Devices.
///
/// This is what an mDNS `_nmos-node._tcp` advertisement points at. It is never
/// called a device: in IS-04 a Device is something a Node contains.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// The fields every NMOS resource carries.
    #[serde(flatten)]
    pub core: ResourceCore,

    /// HTTP access href for the Node's API. Deprecated by IS-04 in favour of
    /// `api.endpoints`, and kept only so the resource round-trips.
    pub href: String,

    /// The Node's hostname, when it reports one. The specs fall back to this
    /// when a Node reports no label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,

    /// Where and in which versions the Node API can be reached.
    pub api: NodeApi,

    /// Capabilities. Not yet defined by the specification, and carried opaquely.
    #[serde(default)]
    pub caps: Capabilities,

    /// Services running on the Node.
    #[serde(default)]
    pub services: Vec<Service>,

    /// Clocks the Node makes available to its Devices.
    #[serde(default)]
    pub clocks: Vec<Clock>,

    /// Network interfaces, which Senders and Receivers bind to by name.
    #[serde(default)]
    pub interfaces: Vec<Interface>,
}

/// Where and in which versions a Node's API can be reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeApi {
    /// API versions this Node supports, written `v1.3` and so on.
    pub versions: Vec<String>,

    /// Every host, port and protocol the API answers on. A Node with two media
    /// interfaces reports more than one, which is why an endpoint is never an
    /// identity — see `DESIGN.md`, "Vocabulary" for the
    /// neighbouring decision and the discovery spec for this one.
    pub endpoints: Vec<ApiEndpoint>,
}

/// One address at which a Node's API answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiEndpoint {
    /// IP address or hostname the API runs on.
    pub host: String,

    /// Port the API runs on.
    pub port: u16,

    /// Whether the API is reached over `http` or `https`.
    pub protocol: Protocol,

    /// Whether this endpoint requires authorization. A Node that does is
    /// reported as unsupported rather than attempted: IS-10 is out of scope,
    /// and a 401 is a worse diagnostic than an honest refusal.
    #[serde(default)]
    pub authorization: bool,
}

/// The scheme an NMOS API is reached over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Http,
    Https,
}

impl Protocol {
    /// The scheme as it appears in a URL.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Protocol::Http => "http",
            Protocol::Https => "https",
        }
    }
}

/// A service running on a Node, identified by a URN.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Service {
    /// URL at which the service is reached.
    pub href: String,

    /// URN identifying the kind of service.
    #[serde(rename = "type")]
    pub kind: String,

    /// Whether the service requires authorization.
    #[serde(default)]
    pub authorization: bool,
}

/// A reference clock a Node offers its Devices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "ref_type", rename_all = "lowercase")]
pub enum Clock {
    /// A clock with no external reference.
    Internal {
        /// Name of this clock, unique within the Node.
        name: String,
    },

    /// A clock referenced to PTP.
    Ptp {
        /// Name of this clock, unique within the Node.
        name: String,
        /// Whether the reference is synchronised to TAI.
        traceable: bool,
        /// Version of PTP in use.
        version: String,
        /// Identifier of the PTP grandmaster.
        gmid: String,
        /// Whether this clock is locked to the reference.
        locked: bool,
    },
}

/// A network interface on a Node, which Senders and Receivers bind to by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interface {
    /// Chassis ID as signalled in LLDP, or null where LLDP is unsuitable.
    pub chassis_id: Option<String>,

    /// Port ID as signalled in LLDP or ARP; a MAC address.
    pub port_id: String,

    /// Name of the interface, unique within the Node.
    pub name: String,

    /// The network device this interface is attached to, when LLDP reports one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attached_network_device: Option<AttachedNetworkDevice>,
}

/// The switch port a Node's interface is plugged into.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachedNetworkDevice {
    /// Chassis ID of the attached network device.
    pub chassis_id: String,

    /// Port ID of the attached network device.
    pub port_id: String,
}
