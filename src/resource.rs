//! The fields every NMOS resource carries, and the two scalar types that give
//! them meaning.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[cfg(feature = "uuid")]
impl ResourceId {
    /// A random identifier, as UUID version 4.
    ///
    /// For a Node with nothing stable to derive from. Such a Node hands out new
    /// identifiers on every restart, and a controller sees the equipment it had
    /// as gone — which is why [`ResourceId::new_v5`] is usually the one wanted.
    #[must_use]
    pub fn new_v4() -> Self {
        Self::from_uuid_unchecked(uuid::Uuid::new_v4())
    }

    /// An identifier derived from a namespace and a name, as UUID version 5.
    ///
    /// This is how a Node keeps its identifiers across a restart: the same name
    /// gives the same identifier, so a controller's connections survive. Version
    /// 5 always lands inside what IS-04 accepts, so unlike
    /// [`TryFrom<Uuid>`](ResourceId::try_from) this cannot fail.
    #[must_use]
    pub fn new_v5(namespace: &uuid::Uuid, name: &[u8]) -> Self {
        Self::from_uuid_unchecked(uuid::Uuid::new_v5(namespace, name))
    }

    /// The identifier as sixteen bytes.
    ///
    /// Infallible: nothing becomes a `ResourceId` without passing the layout
    /// check in [`FromStr`], so an identifier read off the wire is as much a
    /// UUID as one this crate built.
    #[must_use]
    pub fn as_uuid(&self) -> uuid::Uuid {
        let mut bytes = [0_u8; 16];
        let mut digits = self.0.bytes().filter(|byte| *byte != b'-');
        for byte in &mut bytes {
            // The digits are there and are hexadecimal, or this would not be a
            // `ResourceId`. Zero stands in for an impossibility rather than
            // hiding one: this crate does not abort on equipment's behalf.
            let high = digits.next().map_or(0, hex_value);
            let low = digits.next().map_or(0, hex_value);
            *byte = (high << 4) | low;
        }
        uuid::Uuid::from_bytes(bytes)
    }

    /// The hyphenated lower-case form, which is what IS-04 asks for.
    ///
    /// Private, and used only where the version is known to be one the
    /// specification admits — the two constructors above.
    fn from_uuid_unchecked(uuid: uuid::Uuid) -> Self {
        let mut text = [0_u8; uuid::fmt::Hyphenated::LENGTH];
        Self(uuid.hyphenated().encode_lower(&mut text).into())
    }
}

/// The value of one hexadecimal digit.
#[cfg(feature = "uuid")]
fn hex_value(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        _ => 0,
    }
}

/// Any UUID a Node might hold, checked against what IS-04 accepts.
///
/// The nil and max UUIDs carry no version and are refused; everything the
/// standard constructors produce passes.
#[cfg(feature = "uuid")]
impl TryFrom<uuid::Uuid> for ResourceId {
    type Error = ParseError;

    fn try_from(uuid: uuid::Uuid) -> Result<Self, Self::Error> {
        let mut text = [0_u8; uuid::fmt::Hyphenated::LENGTH];
        uuid.hyphenated().encode_lower(&mut text).parse()
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
    /// A version at a given point on the TAI timescale.
    ///
    /// The reading side of the protocol never calls this — versions arrive on
    /// the wire. A Node serving its own resources cannot work without it, which
    /// is why it is here rather than in whatever is doing the serving.
    #[must_use]
    pub const fn new(seconds: u64, nanoseconds: u64) -> Self {
        Self {
            seconds,
            nanoseconds,
        }
    }

    /// Now, as this machine's clock reports it.
    ///
    /// The clock is the system's, so this is UTC-derived rather than true TAI;
    /// NMOS wants TAI, and a Node that needs the distinction must supply its own
    /// timestamp through [`Version::new`]. Two resources stamped from the same
    /// clock still compare correctly, which is what the field is for.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Version`] if the clock is set before the Unix
    /// epoch, which no timestamp on this timescale can express.
    pub fn now() -> Result<Self, ParseError> {
        Self::from_system_time(SystemTime::now())
    }

    /// A version from a point in time.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Version`] if the time is before the Unix epoch.
    pub fn from_system_time(time: SystemTime) -> Result<Self, ParseError> {
        let since = time
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ParseError::Version {
                reason: "a time before the Unix epoch has no version",
            })?;
        Ok(Self {
            seconds: since.as_secs(),
            nanoseconds: u64::from(since.subsec_nanos()),
        })
    }

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
