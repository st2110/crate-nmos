# Changelog

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
