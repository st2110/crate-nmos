# nmos

AMWA NMOS for SMPTE ST 2110 networks, in Rust: the IS-04 resource model, IS-05
connection management, and clients for both.

The types are written by hand and held to the published AMWA JSON Schemas by the
test suite, in both directions. The crate knows the protocol and nothing else —
it holds no state, makes no policy, and has no opinion about how anything is
displayed.

## Status

Early. The crate models IS-04 and the IS-05 connection documents — enough for a
Node to serve a controller and answer its patches, which `examples/node.rs`
does. What is missing is the client method that *sends* a patch, the Registration
and Query APIs, and SDP. `DESIGN.md` says why, and in which order it matters.

## Features

| feature | default | what it adds |
|---|---|---|
| `client` | yes | HTTP clients for the Node API and the Connection API |
| `uuid` | no | resource identifiers as `Uuid`, for the side that generates them |

Consumers that want the types and nothing else take
`nmos = { version = "0.2", default-features = false }`, which pulls in `serde`
and `thiserror` and no I/O at all.

## Contributing device responses

Every NMOS implementation differs in some detail, and no amount of schema
validation catches that. If this crate mishandles your equipment, a captured
response is the most useful thing you can send: it becomes a fixture under
`schemas/examples/`, and from then on the behaviour cannot regress.

## Examples

Two of them, one on each side of the protocol. Run both and they find each other.

### A Node

```sh
cargo run --example node -- 8080 10.0.0.5
```

```text
Node API   http://10.0.0.5:8080/x-nmos/node/v1.3/
Connection http://10.0.0.5:8080/x-nmos/connection/v1.1/
announced as _nmos-node._tcp.local. on 10.0.0.5
```

It exposes two Devices — `Ingest`, with Receivers for video, audio and metadata,
and `Playout`, with the matching Senders and the Sources and Flows behind them —
announces itself over mDNS, serves the IS-04 Node API and the IS-05 Connection
API, and applies the patches a controller sends it.

**Give it the address of the interface the Node lives on.** Both arguments are
optional and default to port 8080 on every interface the mDNS daemon can see,
which on a machine with a VPN or a virtual bridge includes addresses nothing can
reach — a controller that picks one of those finds a Node that never answers.
Real products ask the operator which interface to use for the same reason.

### A controller

```sh
cargo run --example nmosctl -- list
```

```text
nmos example node — http://10.0.0.5:8080 (v1.3)
    Ingest — 0 senders, 3 receivers
    Playout — 3 senders, 0 receivers
```

`list` browses for `_nmos-node._tcp`, takes the API versions each Node
advertises in its TXT record, agrees one both ends understand, and reads the
resource tree. A Node that cannot be read gets its own line with the reason
rather than stopping the listing: an operator asking what is on the network
needs the whole answer, including the broken part of it.

Add a number to listen longer — `nmosctl list 10`.

### Connecting a Receiver

This is what the Connection API is for, and it is worth doing by hand once. With
the Node running, take a Receiver's identifier:

```sh
BASE=http://10.0.0.5:8080
RX=$(curl -s $BASE/x-nmos/node/v1.3/receivers | jq -r '.[0].id')
CONN=$BASE/x-nmos/connection/v1.1/single/receivers/$RX
```

Ask what it will accept:

```sh
curl -s $CONN/constraints | jq -c '.[0]'
```

```json
{"destination_port":{"maximum":65535,"minimum":1},"interface_ip":{"enum":["10.0.0.1"]},
 "multicast_ip":{},"rtp_enabled":{},"source_ip":{}}
```

The interface is a one-element enumeration on purpose: which wire a multicast
group arrives on belongs to the Node, and a controller naming somebody else's
address would not make the traffic appear there.

Now connect it:

```sh
curl -s -X PATCH -H 'Content-Type: application/json' -d '{
  "master_enable": true,
  "sender_id": "aaaa1111-0000-4000-8000-000000000009",
  "transport_params": [{"multicast_ip": "239.10.10.10", "destination_port": 5004}],
  "activation": {"mode": "activate_immediate"}
}' $CONN/staged
```

### The four states of a parameter, seen from outside

The patch above is where `Param` earns its shape. Send these in order and watch
one field at a time:

| patch | what it means | `multicast_ip` after |
|---|---|---|
| `{"multicast_ip": "239.10.10.10"}` | use this | `239.10.10.10` |
| `{"destination_port": 5006}` | *not mentioned* — leave it | `239.10.10.10` |
| `{"multicast_ip": null}` | clear it | `null` |
| `{"destination_port": "auto"}` | you decide | unchanged; the port becomes `"auto"` |

Two states cannot express that. `auto` is kept as `auto` rather than folded into
"no value", because the schema allows a number or `"auto"` for a port and
forbids `null` outright — a Node that folded them would answer with a document
its own specification rejects.

Activation is refused rather than faked when it cannot be honoured:

```sh
curl -s -X PATCH -H 'Content-Type: application/json' \
  -d '{"activation":{"mode":"activate_scheduled_absolute"}}' $CONN/staged
```

```text
501  {"code":501,"debug":null,"error":"this Node activates immediately or not at all"}
```

Scheduling needs a clock shared with the controller and a queue of pending
edits. Accepting and forgetting would be worse than saying no.

### One state, two APIs

Ask both sides about the Receiver you just connected:

```sh
curl -s $BASE/x-nmos/node/v1.3/receivers | jq -c '.[0].subscription'
curl -s $CONN/active | jq -c '{master_enable, sender_id}'
```

```json
{"active":true,"sender_id":"aaaa1111-0000-4000-8000-000000000009"}
{"master_enable":true,"sender_id":"aaaa1111-0000-4000-8000-000000000009"}
```

The same fact appears in both because a controller reads *state* from the IS-04
resource tree — one request covers a whole Node — and goes to IS-05 only for the
addresses. A Node that updated only IS-05 would be connected and look idle.

The example does not keep the fact twice. IS-05 holds it; IS-04 projects it when
asked. Disconnect the Receiver and the difference shows:

```json
{"active":false,"sender_id":null}
{"master_enable":false,"sender_id":"aaaa1111-0000-4000-8000-000000000009"}
```

IS-05 still remembers what it is configured for. IS-04 says it is receiving from
nobody, and nulls the identifier because its schema requires exactly that.

Where that state lives is the application's business. This library models the
documents and has no opinion about who holds them: the example uses a map behind
a mutex, a real Node would use its own configuration.

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
