# Changelog

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0]

### Added

- The `uuid` feature, which until now was declared and empty: it named itself
  "resource identifiers as `Uuid`" and added no code. `ResourceId` gains
  `new_v5` and `new_v4`, `TryFrom<Uuid>` and `as_uuid`. `new_v5` is the one a
  Node wants — identifiers derived from names survive a restart, so a
  controller's connections do — and it cannot fail, which is the point of
  having it rather than a conversion.

## [1.0.0]

The connection half of the protocol, and the examples that prove both halves
work against each other.

### Added

- **The IS-05 connection documents**, in `nmos::is05`: `Param`, `Activation`,
  `ActivationMode`, `TransportFile`, `Constraint`, and the staged patches for
  Senders and Receivers. Both ends of the protocol read these — a controller
  composes a patch, a Node parses it and answers.
- **Transport parameters for every family the vendored schemas describe**: RTP,
  websocket and MQTT, plus `UnknownParams` for a transport this crate does not
  model. The staged patches are generic in the family, because the documents
  themselves never say which one they belong to. See `DESIGN.md`.
- `Version::new`, `Version::now` and `Version::from_system_time`, so a Node can
  stamp the resources it serves rather than only read versions off the wire.
- `is04` and `is05` module maps, each linking the specification it covers, so
  that "which of these types comes from where" has a readable answer. The IS-04
  resources stay re-exported at the crate root.
- `examples/node.rs` — a Node that announces itself over mDNS, serves IS-04 with
  Sources and Flows behind its Senders, serves IS-05, and applies the patches a
  controller sends it. It keeps one state and renders both APIs from it.
- `examples/nmosctl.rs` — the other side: `nmosctl list` browses for Nodes,
  agrees a version with each and reads its resource tree.

### Changed

- The private `AutoOr` used by the connection client is gone; the client now
  reads the same `Param` a patch is written with. One representation of "a value
  or `auto`", not two.
- The read-only contract is checked against the clients rather than the whole
  crate. Naming a document is not sending one, and the model has to name
  activation for a Node to answer a patch at all.
- The published package now carries the test suite and the vendored schemas, so
  the claim that these types round-trip through the AMWA schemas is one a reader
  can run rather than take on trust.

### Removed

- The `discovery` feature. It pulled in `mdns-sd` and added no code, so enabling
  it changed nothing while the README promised it found Nodes on the network.

### Not here yet

Sending a write from the client, the Registration and Query APIs, SDP, and
IS-08. All additive; `DESIGN.md` says in which order they matter.

## [0.1.0]

The IS-04 resource model — Node, Device, Sender, Receiver, Source and Flow —
written by hand and held to the vendored AMWA schemas in both directions, with
read-only HTTP clients for the Node API and the Connection API and version
negotiation usable from either end.
