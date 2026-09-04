//! The identifier and the version are the two scalars every resource carries,
//! and they are parsed straight off the network. Hand-picked cases cannot cover
//! that input space, so the property here is the one that matters for a
//! controller driving live equipment: **whatever arrives, the parse returns,
//! and it returns an error rather than aborting the process.**

// This file is test code in its entirety.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use nmos::{MediaType, ResourceId, Version};
use proptest::prelude::*;

/// Text shaped roughly like an identifier, so the generator spends its time
/// near the boundary rather than on obviously unrelated strings.
fn identifier_ish() -> impl Strategy<Value = String> {
    prop_oneof![
        "[0-9a-fA-F-]{0,64}",
        "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
        "[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}",
        any::<String>(),
    ]
}

fn version_ish() -> impl Strategy<Value = String> {
    prop_oneof![
        "[0-9:]{0,40}",
        "[0-9]{1,25}:[0-9]{1,25}",
        r"[+-]?[0-9]{0,20}[.:]?[0-9]{0,20}",
        any::<String>(),
    ]
}

proptest! {
    #[test]
    fn parsing_an_identifier_never_panics(text in identifier_ish()) {
        let _ = text.parse::<ResourceId>();
    }

    #[test]
    fn parsing_a_version_never_panics(text in version_ish()) {
        let _ = text.parse::<Version>();
    }

    #[test]
    fn parsing_a_media_type_never_panics(text in any::<String>()) {
        let _ = text.parse::<MediaType>();
    }

    /// An identifier that parses prints back exactly as it arrived, which is
    /// what makes the round-trip validation against the published schemas mean
    /// anything.
    #[test]
    fn an_accepted_identifier_round_trips(text in identifier_ish()) {
        if let Ok(id) = text.parse::<ResourceId>() {
            prop_assert_eq!(id.as_str(), text.as_str());
        }
    }

    /// Only the specification's own shape is accepted: 36 characters, lower
    /// case, version 1-5, variant 8-b.
    #[test]
    fn an_accepted_identifier_matches_the_specification(text in identifier_ish()) {
        if text.parse::<ResourceId>().is_ok() {
            let bytes = text.as_bytes();
            prop_assert_eq!(bytes.len(), 36);
            for (position, byte) in bytes.iter().enumerate() {
                if [8, 13, 18, 23].contains(&position) {
                    prop_assert_eq!(*byte, b'-');
                } else {
                    prop_assert!(byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
                }
            }
            prop_assert!((b'1'..=b'5').contains(&bytes[14]));
            prop_assert!(matches!(bytes[19], b'8' | b'9' | b'a' | b'b'));
        }
    }

    /// A version that parses re-prints as the same instant, even when the
    /// original carried leading zeroes.
    #[test]
    fn an_accepted_version_re_parses_to_itself(text in version_ish()) {
        if let Ok(version) = text.parse::<Version>() {
            let printed = version.to_string();
            let reparsed = printed.parse::<Version>();
            prop_assert_eq!(reparsed.ok(), Some(version));
        }
    }

    /// Ordering is by instant, so a later second always outranks an earlier
    /// one whatever the nanoseconds say.
    #[test]
    fn versions_order_by_seconds_then_nanoseconds(
        a_seconds in 0u64..1_000_000,
        a_nanos in 0u64..1_000_000_000,
        b_seconds in 0u64..1_000_000,
        b_nanos in 0u64..1_000_000_000,
    ) {
        let a: Version = format!("{a_seconds}:{a_nanos}").parse().expect("well formed");
        let b: Version = format!("{b_seconds}:{b_nanos}").parse().expect("well formed");
        prop_assert_eq!(a < b, (a_seconds, a_nanos) < (b_seconds, b_nanos));
    }

    /// Oversized input is rejected, not truncated into a plausible value and
    /// not aborted on.
    #[test]
    fn an_oversized_version_is_rejected(digits in 20usize..80) {
        let text = format!("{}:0", "9".repeat(digits));
        prop_assert!(text.parse::<Version>().is_err());
    }

    /// Length alone never makes an identifier acceptable.
    #[test]
    fn an_identifier_of_the_wrong_length_is_always_rejected(length in 0usize..80) {
        prop_assume!(length != 36);
        let text = "a".repeat(length);
        prop_assert!(text.parse::<ResourceId>().is_err());
    }
}
