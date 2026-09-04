//! The errors this crate returns.

use thiserror::Error;

/// A value that does not satisfy the NMOS contract.
///
/// Parsing is where a document from the network becomes a domain value, so it
/// is where every rejection is named: nothing further in the crate has to
/// re-check what a `ResourceId` or a `Version` holds.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// The text is not a resource identifier in the form IS-04 requires.
    #[error("not an NMOS resource identifier: {reason}")]
    ResourceId {
        /// What specifically is wrong, for a message an operator can act on.
        reason: &'static str,
    },

    /// The text is not a TAI version timestamp of the form `<seconds>:<nanoseconds>`.
    #[error("not a TAI version timestamp: {reason}")]
    Version {
        /// What specifically is wrong.
        reason: &'static str,
    },

    /// The text is not a format URN this specification defines.
    #[error("not an NMOS format URN")]
    Format,

    /// The text is not a media type of the form `type/subtype`.
    #[error("not a media type")]
    MediaType,

    /// The text is not an API version of the form `v<major>.<minor>`.
    #[error("not an NMOS API version")]
    ApiVersion,
}
