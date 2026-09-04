//! The API versions this project speaks.

use std::fmt;
use std::str::FromStr;

use crate::error::ParseError;

/// An NMOS API version, written `v<major>.<minor>`.
///
/// Ordering is by major then minor, which is what makes "the highest version
/// both ends understand" a `max` rather than a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiVersion {
    major: u16,
    minor: u16,
}

impl ApiVersion {
    /// Construct a version from its parts.
    #[must_use]
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// The major component.
    #[must_use]
    pub const fn major(self) -> u16 {
        self.major
    }

    /// The minor component.
    #[must_use]
    pub const fn minor(self) -> u16 {
        self.minor
    }
}

impl FromStr for ApiVersion {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let reject = || ParseError::ApiVersion;

        let rest = text.strip_prefix('v').ok_or_else(reject)?;
        let (major, minor) = rest.split_once('.').ok_or_else(reject)?;

        let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
        if !digits(major) || !digits(minor) {
            return Err(reject());
        }

        Ok(Self {
            major: major.parse().map_err(|_| reject())?,
            minor: minor.parse().map_err(|_| reject())?,
        })
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}.{}", self.major, self.minor)
    }
}

/// Pick the highest version both ends understand.
///
/// Returns `None` when the two lists do not overlap. A caller reporting that to
/// a person should name what the other end offered: "unsupported" without the
/// list tells an operator nothing.
///
/// Both ends of the protocol need this. A controller negotiates down to what a
/// Node offers; a Node serving the API negotiates down to what its caller asks
/// for. See `DESIGN.md`.
pub fn negotiate(offered: &[ApiVersion], supported: &[ApiVersion]) -> Option<ApiVersion> {
    offered
        .iter()
        .filter(|version| supported.contains(version))
        .max()
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_highest_common_version_wins_whatever_the_order() {
        let supported = [ApiVersion::new(1, 2), ApiVersion::new(1, 3)];
        let offered = [
            ApiVersion::new(1, 3),
            ApiVersion::new(1, 0),
            ApiVersion::new(1, 2),
        ];
        assert_eq!(negotiate(&offered, &supported), Some(ApiVersion::new(1, 3)));
    }

    #[test]
    fn no_overlap_is_no_version() {
        let supported = [ApiVersion::new(1, 3)];
        assert_eq!(negotiate(&[ApiVersion::new(1, 0)], &supported), None);
        assert_eq!(negotiate(&[], &supported), None);
    }

    #[test]
    fn versions_order_numerically_not_lexically() {
        // `v1.10` is above `v1.9`, which a string comparison would get wrong.
        assert!(ApiVersion::new(1, 9) < ApiVersion::new(1, 10));
        assert!(ApiVersion::new(1, 10) < ApiVersion::new(2, 0));
    }
}
