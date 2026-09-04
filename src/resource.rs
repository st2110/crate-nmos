//! The fields every NMOS resource carries, and the two scalar types that give
//! them meaning.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::ParseError;

/// The globally unique identifier of an NMOS resource.
///
/// IS-04 narrows RFC 4122 further than a general UUID parser does — lower case,
/// version 1 to 5, variant 8 to b — so the check here is the specification's
/// own pattern rather than a general one. Holding the text as it arrived also
/// means a resource serializes back byte-for-byte, which is what makes the
/// round-trip validation meaningful.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(Box<str>);

impl ResourceId {
    /// The identifier as it appears on the wire.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ResourceId {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        const LAYOUT: [usize; 4] = [8, 13, 18, 23];

        let reject = |reason| Err(ParseError::ResourceId { reason });
        let bytes = text.as_bytes();

        if bytes.len() != 36 {
            return reject("expected 36 characters");
        }

        for (position, byte) in bytes.iter().enumerate() {
            let expected_hyphen = LAYOUT.contains(&position);
            if expected_hyphen {
                if *byte != b'-' {
                    return reject("expected a hyphen at 8, 13, 18 and 23");
                }
                continue;
            }
            if !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase() {
                return reject("expected lower-case hexadecimal digits");
            }
        }

        // The version nibble opens the third group, the variant nibble the
        // fourth. IS-04 admits neither the nil UUID nor versions beyond 5.
        match bytes.get(14) {
            Some(b'1'..=b'5') => {}
            _ => return reject("version nibble outside 1-5"),
        }
        match bytes.get(19) {
            Some(b'8' | b'9' | b'a' | b'b') => {}
            _ => return reject("variant nibble outside 8-b"),
        }

        Ok(Self(text.into()))
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for ResourceId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ResourceId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // `Cow` rather than `&str`: a `serde_json::Value` cannot lend a
        // borrowed string, and the examples are parsed from one in the tests.
        let text = Cow::<str>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// When a resource last changed: a TAI timestamp written `<seconds>:<nanoseconds>`.
///
/// Held as numbers rather than text because comparing two versions is the
/// point of the field — it is how the controller notices that a resource it
/// already holds has moved on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    seconds: u64,
    nanoseconds: u64,
}

impl Version {
    /// Whole seconds since the TAI epoch.
    #[must_use]
    pub fn seconds(self) -> u64 {
        self.seconds
    }

    /// Nanoseconds within the second.
    #[must_use]
    pub fn nanoseconds(self) -> u64 {
        self.nanoseconds
    }
}

impl FromStr for Version {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let reject = |reason| ParseError::Version { reason };

        let (seconds, nanoseconds) = text
            .split_once(':')
            .ok_or_else(|| reject("expected <seconds>:<nanoseconds>"))?;

        // `str::parse` accepts a leading `+`, which the specification's pattern
        // does not, so the digits are checked before they are read.
        let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
        if !digits(seconds) || !digits(nanoseconds) {
            return Err(reject("expected decimal digits either side of the colon"));
        }

        Ok(Self {
            seconds: seconds.parse().map_err(|_| reject("seconds do not fit"))?,
            nanoseconds: nanoseconds
                .parse()
                .map_err(|_| reject("nanoseconds do not fit"))?,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.seconds, self.nanoseconds)
    }
}

impl Serialize for Version {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = Cow::<str>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// An object the specification leaves undefined — a resource's `caps`, so far.
///
/// Carried verbatim rather than dropped, so that a resource this project reads
/// and writes back satisfies its schema unchanged. Nothing here is interpreted.
pub type Capabilities = BTreeMap<String, serde_json::Value>;

/// Freeform tags a resource carries, each key mapping to a list of values.
///
/// Ordered rather than hashed so that a serialized resource is byte-stable,
/// which keeps the round-trip tests and any snapshot deterministic.
pub type Tags = BTreeMap<String, Vec<String>>;

/// The fields `resource_core.json` gives every NMOS resource.
///
/// `label`, `description` and `tags` are required by the specification but
/// defaulted here, because equipment omits them and a Node that omits its label
/// must still be listed — under its hostname — rather than dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceCore {
    /// Globally unique identifier for the resource.
    pub id: ResourceId,

    /// When an attribute of the resource last changed.
    pub version: Version,

    /// Freeform label. Repeats across resources on real equipment, so it is
    /// never enough on its own to tell two resources apart.
    #[serde(default)]
    pub label: String,

    /// Freeform description.
    #[serde(default)]
    pub description: String,

    /// Freeform tags.
    #[serde(default)]
    pub tags: Tags,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identifier_layout_check_looks_at_every_position() {
        // A hyphen in the wrong place must not slip through by luck.
        assert!(
            "3b8be755-08ff-452b-b217-c9151eb2-1193"
                .parse::<ResourceId>()
                .is_err()
        );
        assert!(
            "3b8be7550-8ff-452b-b217-c9151eb21193"
                .parse::<ResourceId>()
                .is_err()
        );
    }

    #[test]
    fn a_version_with_leading_zeroes_parses_and_normalises() {
        let version: Version = "007:0000".parse().expect("the schema pattern allows this");
        assert_eq!(version.to_string(), "7:0");
    }

    #[test]
    fn a_version_with_a_sign_is_rejected() {
        // `u64::from_str` would accept `+7`; the specification's pattern does not.
        assert!("+7:0".parse::<Version>().is_err());
    }
}
