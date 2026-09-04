//! The Source resource: the logical origin of a Flow, upstream of the Flow
//! itself. Never a synonym for Sender.

use serde::{Deserialize, Serialize};

use crate::media::Rate;
use crate::resource::{Capabilities, ResourceCore, ResourceId};

/// The fields every Source carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCore {
    /// The fields every NMOS resource carries.
    #[serde(flatten)]
    pub core: ResourceCore,

    /// Maximum grains per second for Flows derived from this Source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grain_rate: Option<Rate>,

    /// Capabilities. Not yet defined by the specification.
    #[serde(default)]
    pub caps: Capabilities,

    /// The Device this Source was created by.
    pub device_id: ResourceId,

    /// Sources that came together at the input to this one.
    #[serde(default)]
    pub parents: Vec<ResourceId>,

    /// The Node clock this Source is referenced to, when it names one.
    pub clock_name: Option<String>,
}

/// The logical origin of a Flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "format")]
pub enum Source {
    /// A video Source.
    #[serde(rename = "urn:x-nmos:format:video")]
    Video {
        /// Fields shared by every Source.
        #[serde(flatten)]
        core: SourceCore,
    },

    /// A mux Source.
    #[serde(rename = "urn:x-nmos:format:mux")]
    Mux {
        /// Fields shared by every Source.
        #[serde(flatten)]
        core: SourceCore,
    },

    /// An audio Source, which names its channels.
    #[serde(rename = "urn:x-nmos:format:audio")]
    Audio {
        /// Fields shared by every Source.
        #[serde(flatten)]
        core: SourceCore,
        /// The audio channels this Source carries.
        channels: Vec<Channel>,
    },

    /// A data Source.
    #[serde(rename = "urn:x-nmos:format:data")]
    Data {
        /// Fields shared by every Source.
        #[serde(flatten)]
        core: SourceCore,
        /// The event type generated, if applicable.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        event_type: Option<String>,
    },
}

impl Source {
    /// The fields every Source carries.
    #[must_use]
    pub fn core(&self) -> &SourceCore {
        match self {
            Source::Video { core }
            | Source::Mux { core }
            | Source::Audio { core, .. }
            | Source::Data { core, .. } => core,
        }
    }

    /// The identifier of this Source.
    #[must_use]
    pub fn id(&self) -> &ResourceId {
        &self.core().core.id
    }
}

/// One audio channel of a Source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel {
    /// Free-text label for the channel.
    pub label: String,

    /// Symbol for the channel, from VSF TR-03 Appendix A.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}
