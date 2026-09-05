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
| `uuid` | no | resource identifiers as `Uuid`, for the side that generates them |

Consumers that want the types and nothing else take
`nmos = { version = "0.1", default-features = false }`, which pulls in `serde`
and `thiserror` and no I/O at all.

## Contributing device responses

Every NMOS implementation differs in some detail, and no amount of schema
validation catches that. If this crate mishandles your equipment, a captured
response is the most useful thing you can send: it becomes a fixture under
`schemas/examples/`, and from then on the behaviour cannot regress.

## Examples

```sh
cargo run --example node
```

Runs a Node: two Devices, one receiving video, audio and metadata and one
sending them, with the Sources and Flows behind them. It announces itself over
mDNS, answers the IS-04 Node API, and accepts the IS-05 `PATCH` a controller
uses to connect a Receiver.

Pass the address of the interface the Node lives on — `cargo run --example node
-- 8080 10.0.0.5`. Without one it announces every interface, which on a machine
with a VPN includes addresses nothing can reach.

Two things in it are worth reading. `apply` is where a patch meets the four
states of a transport parameter, which is the clearest explanation of why
`Param` is not `Option<Option<T>>`. And the Node keeps **one** state: IS-05
serves it almost verbatim while IS-04 projects the part it describes, so the two
APIs cannot disagree about whether something is connected.

Serving HTTP and announcing over mDNS are not this library's job — it models the
protocol and has no opinion about who holds the state.

```sh
cargo run --example nmosctl -- list
```

The other side: browses for Nodes, agrees an API version with each, reads its
resource tree and reports what it found — including which Nodes could not be
read, because an operator asking what is on the network needs the whole answer.

A full controller ([jackfield](https://github.com/st2110/jackfield)) keeps the
inventory, follows changes and draws a screen. `nmosctl` is the shape of the
client underneath that.

## What the published crate contains

The library, and the suite that checks it. The vendored AMWA schemas ship too,
because the tests are worthless without them — so `cargo test` works on a
downloaded or vendored copy, and the claim that these types round-trip through
the published schemas is one you can verify rather than take on trust.

Only development scaffolding is left out.

## License

Apache-2.0 — see [LICENSE](LICENSE). The vendored AMWA schemas under `schemas/`
are under the same licence, but the copyright there is AMWA's; see
`schemas/LICENSE`, `schemas/NOTICE` and `schemas/PROVENANCE.md`.
