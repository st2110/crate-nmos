//! The Flow resource: the media a Sender carries.
//!
//! IS-04 models this as eight sibling schemas selected by `format` and
//! `media_type`. Here it is one Rust enum, because that is what it is, and
//! because the media type is the field that tells two identically labelled
//! Senders apart — the reason `node-inventory` resolves the Sender-to-Flow
//! reference at all.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::media::{Format, MediaType, Rate};
use crate::resource::{ResourceCore, ResourceId};

/// The fields every Flow carries, whatever it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowCore {
    /// The fields every NMOS resource carries.
    pub core: ResourceCore,

    /// The Source this Flow originates from.
    pub source_id: ResourceId,

    /// The Device this Flow was created by.
    pub device_id: ResourceId,

    /// Flows that came together to make this one.
    pub parents: Vec<ResourceId>,

    /// Grains per second, for a periodic Flow.
    pub grain_rate: Option<Rate>,
}

/// One component plane of a raw video Flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    /// Which plane this is.
    pub name: ComponentName,
    /// Width in pixels.
    pub width: i64,
    /// Height in pixels.
    pub height: i64,
    /// Bits per sample.
    pub bit_depth: i64,
}

/// The plane a video component describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs, reason = "each variant is the specification's own label")]
pub enum ComponentName {
    Y,
    Cb,
    Cr,
    I,
    Ct,
    Cp,
    A,
    R,
    G,
    B,
    DepthMap,
}

/// How the frames of a video Flow are scanned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs, reason = "each variant is the specification's own label")]
pub enum InterlaceMode {
    #[default]
    Progressive,
    InterlacedTff,
    InterlacedBff,
    InterlacedPsf,
}

/// The picture geometry and colour a video Flow shares across its variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoCore {
    /// Width of the picture in pixels.
    pub frame_width: i64,
    /// Height of the picture in pixels.
    pub frame_height: i64,
    /// How the frames are scanned.
    pub interlace_mode: InterlaceMode,
    /// Colorspace used for the video.
    pub colorspace: String,
    /// Transfer characteristic.
    pub transfer_characteristic: Option<String>,
}

/// A pair of SDI ancillary data identification words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DidSdid {
    /// Data identification word.
    #[serde(rename = "DID", default, skip_serializing_if = "Option::is_none")]
    pub did: Option<String>,
    /// Secondary data identification word.
    #[serde(rename = "SDID", default, skip_serializing_if = "Option::is_none")]
    pub sdid: Option<String>,
}

/// The media a Sender carries.
///
/// The variant is chosen by format first and media type second, and never by
/// which schema happens to match: `flow.json` is an `anyOf`, so more than one
/// sibling can accept the same document, and a controller that picked whichever
/// matched first would report a raw audio Flow as coded depending on field
/// order.
///
/// Deliberately exhaustive: the engine and the interface both match on every
/// variant, and a new one should break the build rather than fall into a
/// catch-all that renders as "unknown".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Flow {
    /// Uncompressed video: `video/raw`.
    VideoRaw {
        /// Fields shared by every Flow.
        core: FlowCore,
        /// Picture geometry and colour.
        video: VideoCore,
        /// Media type, always `video/raw`.
        media_type: MediaType,
        /// The component planes.
        components: Vec<Component>,
    },

    /// Compressed video: anything under `video/` that is not `video/raw`.
    VideoCoded {
        /// Fields shared by every Flow.
        core: FlowCore,
        /// Picture geometry and colour.
        video: VideoCore,
        /// Media type.
        media_type: MediaType,
    },

    /// Uncompressed linear audio: `audio/L16`, `audio/L24` and their kin.
    AudioRaw {
        /// Fields shared by every Flow.
        core: FlowCore,
        /// Samples per second.
        sample_rate: Rate,
        /// Media type.
        media_type: MediaType,
        /// Bits per sample.
        bit_depth: Option<i64>,
    },

    /// Compressed audio.
    AudioCoded {
        /// Fields shared by every Flow.
        core: FlowCore,
        /// Samples per second.
        sample_rate: Rate,
        /// Media type.
        media_type: MediaType,
    },

    /// SDI ancillary data: `video/smpte291`.
    SdiAncData {
        /// Fields shared by every Flow.
        core: FlowCore,
        /// Media type, always `video/smpte291`.
        media_type: MediaType,
        /// Data identification words carried.
        did_sdid: Vec<DidSdid>,
    },

    /// IS-07 event data: `application/json`.
    JsonData {
        /// Fields shared by every Flow.
        core: FlowCore,
        /// Media type, always `application/json`.
        media_type: MediaType,
        /// The event type carried, when the Flow declares one.
        event_type: Option<String>,
    },

    /// Any other data Flow.
    Data {
        /// Fields shared by every Flow.
        core: FlowCore,
        /// Media type.
        media_type: MediaType,
    },

    /// A muxed Flow, such as `video/SMPTE2022-6`.
    Mux {
        /// Fields shared by every Flow.
        core: FlowCore,
        /// Media type.
        media_type: MediaType,
    },
}

impl Flow {
    /// The fields every Flow carries.
    #[must_use]
    pub fn core(&self) -> &FlowCore {
        match self {
            Flow::VideoRaw { core, .. }
            | Flow::VideoCoded { core, .. }
            | Flow::AudioRaw { core, .. }
            | Flow::AudioCoded { core, .. }
            | Flow::SdiAncData { core, .. }
            | Flow::JsonData { core, .. }
            | Flow::Data { core, .. }
            | Flow::Mux { core, .. } => core,
        }
    }

    /// What this Flow carries. The field an operator reads to tell two Senders
    /// under one label apart.
    #[must_use]
    pub fn media_type(&self) -> &MediaType {
        match self {
            Flow::VideoRaw { media_type, .. }
            | Flow::VideoCoded { media_type, .. }
            | Flow::AudioRaw { media_type, .. }
            | Flow::AudioCoded { media_type, .. }
            | Flow::SdiAncData { media_type, .. }
            | Flow::JsonData { media_type, .. }
            | Flow::Data { media_type, .. }
            | Flow::Mux { media_type, .. } => media_type,
        }
    }

    /// The high-level format of this Flow.
    #[must_use]
    pub fn format(&self) -> Format {
        match self {
            Flow::VideoRaw { .. } | Flow::VideoCoded { .. } => Format::Video,
            Flow::AudioRaw { .. } | Flow::AudioCoded { .. } => Format::Audio,
            Flow::SdiAncData { .. } | Flow::JsonData { .. } | Flow::Data { .. } => Format::Data,
            Flow::Mux { .. } => Format::Mux,
        }
    }

    /// The identifier of this Flow.
    #[must_use]
    pub fn id(&self) -> &ResourceId {
        &self.core().core.id
    }
}

/// The flat document a Flow is on the wire, in every variant at once.
///
/// Serde cannot key an enum on two fields, and `flow.json` needs both, so the
/// document is read into this and then dispatched. It is also what a Flow
/// serializes through, which keeps one description of the wire format rather
/// than two that can drift.
#[derive(Debug, Serialize, Deserialize)]
struct Wire {
    #[serde(flatten)]
    core: ResourceCore,
    source_id: ResourceId,
    device_id: ResourceId,
    #[serde(default)]
    parents: Vec<ResourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    grain_rate: Option<Rate>,
    format: Format,
    media_type: MediaType,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    frame_width: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    frame_height: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    interlace_mode: Option<InterlaceMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    colorspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    transfer_characteristic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    components: Option<Vec<Component>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    sample_rate: Option<Rate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bit_depth: Option<i64>,

    #[serde(rename = "DID_SDID", default, skip_serializing_if = "Option::is_none")]
    did_sdid: Option<Vec<DidSdid>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    event_type: Option<String>,
}

impl Wire {
    fn flow_core(&self) -> FlowCore {
        FlowCore {
            core: self.core.clone(),
            source_id: self.source_id.clone(),
            device_id: self.device_id.clone(),
            parents: self.parents.clone(),
            grain_rate: self.grain_rate,
        }
    }

    fn video_core<E: serde::de::Error>(&self) -> Result<VideoCore, E> {
        Ok(VideoCore {
            frame_width: self
                .frame_width
                .ok_or_else(|| E::missing_field("frame_width"))?,
            frame_height: self
                .frame_height
                .ok_or_else(|| E::missing_field("frame_height"))?,
            interlace_mode: self.interlace_mode.unwrap_or_default(),
            colorspace: self
                .colorspace
                .clone()
                .ok_or_else(|| E::missing_field("colorspace"))?,
            transfer_characteristic: self.transfer_characteristic.clone(),
        })
    }
}

impl<'de> Deserialize<'de> for Flow {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = Wire::deserialize(deserializer)?;
        let core = wire.flow_core();
        let media_type = wire.media_type.clone();

        Ok(match wire.format {
            Format::Video if media_type.as_str() == "video/raw" => Flow::VideoRaw {
                core,
                video: wire.video_core::<D::Error>()?,
                media_type,
                components: wire
                    .components
                    .clone()
                    .ok_or_else(|| D::Error::missing_field("components"))?,
            },
            Format::Video => Flow::VideoCoded {
                core,
                video: wire.video_core::<D::Error>()?,
                media_type,
            },
            Format::Audio => {
                let sample_rate = wire
                    .sample_rate
                    .ok_or_else(|| D::Error::missing_field("sample_rate"))?;
                if media_type.is_linear_audio() {
                    Flow::AudioRaw {
                        core,
                        sample_rate,
                        media_type,
                        bit_depth: wire.bit_depth,
                    }
                } else {
                    Flow::AudioCoded {
                        core,
                        sample_rate,
                        media_type,
                    }
                }
            }
            Format::Data => match media_type.as_str() {
                "video/smpte291" => Flow::SdiAncData {
                    core,
                    media_type,
                    did_sdid: wire.did_sdid.clone().unwrap_or_default(),
                },
                "application/json" => Flow::JsonData {
                    core,
                    media_type,
                    event_type: wire.event_type.clone(),
                },
                _ => Flow::Data { core, media_type },
            },
            Format::Mux => Flow::Mux { core, media_type },
        })
    }
}

impl Serialize for Flow {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let core = self.core();
        let mut wire = Wire {
            core: core.core.clone(),
            source_id: core.source_id.clone(),
            device_id: core.device_id.clone(),
            parents: core.parents.clone(),
            grain_rate: core.grain_rate,
            format: self.format(),
            media_type: self.media_type().clone(),
            frame_width: None,
            frame_height: None,
            interlace_mode: None,
            colorspace: None,
            transfer_characteristic: None,
            components: None,
            sample_rate: None,
            bit_depth: None,
            did_sdid: None,
            event_type: None,
        };

        let mut set_video = |video: &VideoCore| {
            wire.frame_width = Some(video.frame_width);
            wire.frame_height = Some(video.frame_height);
            wire.interlace_mode = Some(video.interlace_mode);
            wire.colorspace = Some(video.colorspace.clone());
            wire.transfer_characteristic = video.transfer_characteristic.clone();
        };

        match self {
            Flow::VideoRaw {
                video, components, ..
            } => {
                set_video(video);
                wire.components = Some(components.clone());
            }
            Flow::VideoCoded { video, .. } => set_video(video),
            Flow::AudioRaw {
                sample_rate,
                bit_depth,
                ..
            } => {
                wire.sample_rate = Some(*sample_rate);
                wire.bit_depth = *bit_depth;
            }
            Flow::AudioCoded { sample_rate, .. } => wire.sample_rate = Some(*sample_rate),
            Flow::SdiAncData { did_sdid, .. } => wire.did_sdid = Some(did_sdid.clone()),
            Flow::JsonData { event_type, .. } => wire.event_type = event_type.clone(),
            Flow::Data { .. } | Flow::Mux { .. } => {}
        }

        wire.serialize(serializer)
    }
}
