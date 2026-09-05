//! Resource identifiers built from UUIDs, which is what the generating side of
//! the protocol needs — the `uuid` feature.
//!
//! A Node hands out identifiers it computed; a controller only ever reads them.
//! The reason this is a feature rather than the only way in is that the reading
//! side should not have to compile a UUID library to parse a string.

#![cfg(feature = "uuid")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // tests

use nmos::{ParseError, ResourceId};
use uuid::Uuid;

/// A Node's identifiers have to survive a restart, so they are derived from
/// names rather than handed out at random. Version 5 is that derivation, and it
/// always lands inside what IS-04 accepts — which is why this one cannot fail.
#[test]
fn a_name_derived_identifier_is_stable_and_valid() {
    let namespace = Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"example.invalid");

    let once = ResourceId::new_v5(&namespace, b"device\x1fsdi1");
    let again = ResourceId::new_v5(&namespace, b"device\x1fsdi1");
    let other = ResourceId::new_v5(&namespace, b"device\x1fsdi2");

    assert_eq!(once, again, "the same name gives the same identifier");
    assert_ne!(once, other);
    // Built, not parsed — so prove it would have parsed.
    assert_eq!(once.as_str().parse::<ResourceId>().unwrap(), once);
}

/// The same, for a Node that has no stable names to derive from.
#[test]
fn a_random_identifier_is_valid() {
    let one = ResourceId::new_v4();
    let two = ResourceId::new_v4();

    assert_ne!(one, two);
    assert_eq!(one.as_str().parse::<ResourceId>().unwrap(), one);
}

/// Any UUID the specification admits converts. IS-04 narrows RFC 4122 to
/// versions 1 to 5, so the conversion is fallible even though every UUID the
/// constructors above produce passes it.
#[test]
fn every_versioned_uuid_converts() {
    let cases = [
        Uuid::new_v4(),
        Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"nmos"),
        Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap(),
    ];

    for uuid in cases {
        let id = ResourceId::try_from(uuid).expect("a versioned UUID is an IS-04 identifier");
        assert_eq!(id.as_str(), uuid.hyphenated().to_string());
    }
}

/// The two UUIDs that carry no version are refused rather than let through:
/// a resource identified by the nil UUID fails its own schema, and a Node that
/// serves one is telling a controller something it cannot use.
#[test]
fn the_nil_and_max_uuids_are_refused() {
    for uuid in [Uuid::nil(), Uuid::max()] {
        assert!(
            matches!(
                ResourceId::try_from(uuid),
                Err(ParseError::ResourceId { .. })
            ),
            "{uuid} has no version and must not become an identifier"
        );
    }
}

/// An identifier arrives on the wire lower-case; a UUID formatted upper-case is
/// not the same string, and the conversion must not be the way that gets in.
#[test]
fn the_conversion_writes_lower_case() {
    let uuid = Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"CASE");
    let id = ResourceId::try_from(uuid).expect("a version 5 UUID converts");

    assert_eq!(id.as_str(), id.as_str().to_ascii_lowercase());
    assert_ne!(id.as_str(), uuid.hyphenated().to_string().to_uppercase());
}

/// The way back out. Every `ResourceId` has passed the IS-04 layout check, so
/// it is a UUID whether it was built from one or read off the wire, and a Node
/// that keyed its own state on 16 bytes gets them back without re-parsing.
#[test]
fn an_identifier_converts_back_to_the_uuid_it_came_from() {
    let namespace = Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"example.invalid");
    let cases = [
        Uuid::new_v4(),
        Uuid::new_v5(&namespace, b"receiver\x1fvideo"),
        Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap(),
        Uuid::parse_str("ffffffff-ffff-4fff-bfff-ffffffffffff").unwrap(),
        Uuid::parse_str("00000000-0000-1000-8000-000000000000").unwrap(),
    ];

    for uuid in cases {
        let id = ResourceId::try_from(uuid).expect("a versioned UUID converts");
        assert_eq!(id.as_uuid(), uuid);
    }
}

/// The same for an identifier that was never a `Uuid` in this process — the
/// controller's case, where it arrived as text from a Node.
#[test]
fn an_identifier_read_off_the_wire_converts_to_a_uuid() {
    let id: ResourceId = "3b8be755-08ff-452b-b217-c9151eb21193"
        .parse()
        .expect("a well-formed identifier");

    assert_eq!(
        id.as_uuid(),
        Uuid::parse_str("3b8be755-08ff-452b-b217-c9151eb21193").unwrap()
    );
}
