//! The Device resource: a grouping inside a Node that owns Senders and Receivers.

use serde::{Deserialize, Serialize};

use crate::resource::{ResourceCore, ResourceId};

/// A grouping inside a Node, typically one physical port.
///
/// The bench converter presents three — `SDI 1`, `SDI 2`, `SDI 3` — and the
/// Device is the thing an engineer at the rack is actually thinking about, which
/// is why the interface never flattens Senders and Receivers away from it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    /// The fields every NMOS resource carries.
    #[serde(flatten)]
    pub core: ResourceCore,

    /// Device type URN.
    #[serde(rename = "type")]
    pub kind: String,

    /// The Node that created this Device.
    pub node_id: ResourceId,

    /// Senders attached to the Device. Deprecated by IS-04 in favour of each
    /// Sender's own `device_id`, which is what the inventory resolves by.
    #[serde(default)]
    pub senders: Vec<ResourceId>,

    /// Receivers attached to the Device. Deprecated on the same terms.
    #[serde(default)]
    pub receivers: Vec<ResourceId>,

    /// Control endpoints the Device exposes. This is where the Connection API
    /// base URL is advertised, so it is how the transport pass finds IS-05.
    #[serde(default)]
    pub controls: Vec<Control>,
}

/// A control endpoint a Device exposes, such as its Connection API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Control {
    /// URL at which the control endpoint is reached.
    pub href: String,

    /// URN identifying the control format, for example
    /// `urn:x-nmos:control:sr-ctrl/v1.1` for IS-05.
    #[serde(rename = "type")]
    pub kind: String,

    /// Whether the endpoint requires authorization.
    #[serde(default)]
    pub authorization: bool,
}

impl Device {
    /// The href of the first control matching `urn_prefix`, if the Device
    /// advertises one.
    ///
    /// Control URNs carry a version suffix (`.../v1.1`), so callers match on the
    /// prefix and pick the version themselves.
    #[must_use]
    pub fn control_href(&self, urn_prefix: &str) -> Option<&str> {
        self.controls
            .iter()
            .find(|c| c.kind.starts_with(urn_prefix))
            .map(|c| c.href.as_str())
    }
}
