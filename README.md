# nmos

AMWA NMOS for SMPTE ST 2110 networks, in Rust: the IS-04 resource model, IS-05
connection management, and clients for both.

The types are written by hand and held to the published AMWA JSON Schemas by the
test suite, in both directions. The crate knows the protocol and nothing else —
it holds no state, makes no policy, and has no opinion about how anything is
displayed.

## Status

Early. Today the crate models IS-04 and reads IS-05; it does not yet write
connections, speak to a registry, or parse SDP. `DESIGN.md` says what is missing
and in which order it matters.

## Features

| feature | default | what it adds |
|---|---|---|
| `client` | yes | HTTP clients for the Node API and the Connection API |
| `discovery` | no | finding Nodes on the network over mDNS |
| `uuid` | no | resource identifiers as `Uuid`, for the side that generates them |

Consumers that want the types and nothing else take
`nmos = { version = "0.1", default-features = false }`, which pulls in `serde`
and `thiserror` and no I/O at all.

## Contributing device responses

Every NMOS implementation differs in some detail, and no amount of schema
validation catches that. If this crate mishandles your equipment, a captured
response is the most useful thing you can send: it becomes a fixture under
`schemas/examples/`, and from then on the behaviour cannot regress.

## License

Apache-2.0 — see [LICENSE](LICENSE). The vendored AMWA schemas under `schemas/`
are under the same licence, but the copyright there is AMWA's; see
`schemas/LICENSE`, `schemas/NOTICE` and `schemas/PROVENANCE.md`.
