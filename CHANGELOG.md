# Changelog

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0]

### Added

- The IS-05 connection documents, in `nmos::is05`: `Param`, `Activation`,
  `ActivationMode`, `TransportFile`, `Constraint`, and the staged patches for
  Senders and Receivers with the full RTP transport parameter sets. Both ends of
  the protocol need these — a controller composes a patch, a Node parses one.
- `Version::new`, `Version::now` and `Version::from_system_time`, so a Node can
  stamp the resources it serves rather than only read versions off the wire.

### Changed

- The private `AutoOr` used by the connection client is gone; the client now
  reads the same `Param` a patch is written with. One representation of "a value
  or `auto`", not two.
- The read-only contract is now checked against the clients rather than the
  whole crate. Naming a document is not sending one, and the model has to name
  activation for a Node to be able to answer a patch at all.

### Added

- The IS-04 resource model: Node, Device, Sender, Receiver, Source and Flow,
  written by hand and validated against the vendored AMWA schemas in both
  directions.
- IS-05 connection state, and transport parameters for both legs of a redundant
  pair.
- Read-only HTTP clients for the Node API and the Connection API, behind the
  default `client` feature.
- API version negotiation, usable from either end of the protocol.

### Notes

Nothing is published yet. The first release is expected to be followed closely by
a breaking one: writing connections needs a transport parameter with four states
rather than two, and that will change public types. See `DESIGN.md`.
