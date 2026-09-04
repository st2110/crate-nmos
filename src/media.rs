//! What a Flow carries and what a Receiver accepts.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::ParseError;

/// The high-level kind of content, carried on the wire as a URN.
///
/// A format the specification does not define is rejected rather than mapped to
/// a catch-all: guessing what an unrecognised format means is how a controller
/// ends up telling an operator something untrue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Format {
    Video,
    Audio,
    Data,
    Mux,
}

impl Format {
    /// The URN this format is written as in IS-04.
    #[must_use]
    pub fn as_urn(self) -> &'static str {
        match self {
            Format::Video => "urn:x-nmos:format:video",
            Format::Audio => "urn:x-nmos:format:audio",
            Format::Data => "urn:x-nmos:format:data",
            Format::Mux => "urn:x-nmos:format:mux",
        }
    }
}

impl FromStr for Format {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "urn:x-nmos:format:video" => Ok(Format::Video),
            "urn:x-nmos:format:audio" => Ok(Format::Audio),
            "urn:x-nmos:format:data" => Ok(Format::Data),
            "urn:x-nmos:format:mux" => Ok(Format::Mux),
            _ => Err(ParseError::Format),
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_urn())
    }
}

impl Serialize for Format {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_urn())
    }
}

impl<'de> Deserialize<'de> for Format {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = std::borrow::Cow::<str>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// An IANA media type such as `video/raw` or `audio/L24`.
///
/// This is the field that makes a Sender row usable: equipment presents several
/// Senders under one label and only the media type tells them apart.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaType(Box<str>);

impl MediaType {
    /// The media type as it appears on the wire.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is one of the uncompressed linear audio types, which is what
    /// separates a raw audio Flow from a coded one.
    #[must_use]
    pub fn is_linear_audio(&self) -> bool {
        self.0
            .strip_prefix("audio/L")
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
    }
}

impl FromStr for MediaType {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        // The schemas spell this `^[^\s/]+/[^\s/]+$` — exactly one slash, and no
        // whitespace on either side of it.
        let Some((kind, subtype)) = text.split_once('/') else {
            return Err(ParseError::MediaType);
        };
        let usable = |part: &str| {
            !part.is_empty() && !part.contains('/') && !part.chars().any(char::is_whitespace)
        };
        if !usable(kind) || !usable(subtype) {
            return Err(ParseError::MediaType);
        }
        Ok(Self(text.into()))
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for MediaType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for MediaType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = std::borrow::Cow::<str>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// A rate expressed as a rational number, as IS-04 writes grain and sample rates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rate {
    /// Numerator.
    pub numerator: i64,

    /// Denominator; the specification defaults it to one.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub denominator: i64,
}

fn one() -> i64 {
    1
}

fn is_one(value: &i64) -> bool {
    *value == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_audio_is_told_apart_from_coded_audio() {
        for linear in ["audio/L8", "audio/L16", "audio/L20", "audio/L24"] {
            assert!(
                linear
                    .parse::<MediaType>()
                    .expect("parses")
                    .is_linear_audio()
            );
        }
        for coded in ["audio/AAC", "audio/L", "audio/Lx", "video/raw"] {
            assert!(
                !coded
                    .parse::<MediaType>()
                    .expect("parses")
                    .is_linear_audio()
            );
        }
    }
}
