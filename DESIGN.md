# One model, two consumers

This crate exists because the same NMOS documents were already modelled twice
inside one company, from opposite ends. The design question is not "what does a
controller need" but "what shape serves a Node and a controller without either
one having to fork it".

## The two consumers

**A controller** reads Nodes it did not build. It sees whatever the vendor
emits, must not reject a device over a detail it does not care about, needs the
whole resource set — Senders, Flows and Sources included — and never generates
an identifier.

**A Node** serves its own resources. It generates identifiers and versions,
receives PATCH requests against `staged`, decides whether to accept them, and
schedules activation. It may expose only part of the resource set: a Node that
only receives has no Senders at all.

What each side had before this crate is the argument for it:

| | controller | Node |
|---|---|---|
| IS-04 resources | Node, Device, Sender, Receiver, Flow, Source | Node, Device, Receiver |
| IS-05 reading | transport parameters of both legs | — |
| IS-05 writing | — | staged patch, activation, errors |
| Identifiers | parses whatever arrives | derives them, must be stable |
| Versions | parses | parses and stamps its own |

Neither side is a superset. Each holds exactly what the other lacks, and that is
why one crate is worth more than the sum.

## Decisions

### Identifiers are strings that may be UUIDs

IS-04 says resource identifiers are UUIDs. Real equipment does not always agree,
and a controller that refuses to display a device because its identifier is
malformed is worse than useless — it hides the very device an operator is
looking for.

`ResourceId` is therefore a string newtype: lenient on the way in. The `uuid`
feature adds `From<Uuid>` and `as_uuid()`, which is what the generating side
needs — exact on the way out. Leniency is a property of parsing, not of the
type, and the two sides need opposite defaults.

### Versions can be read and stamped

`Version` is `{ seconds, nanoseconds }` as it appears on the wire, and it also
carries `now()` and `from_system_time()`. A controller never calls those; a Node
cannot work without them. Having both costs nothing and prevents the second
implementation.

### Absent, null, auto and set are four different things

The reading side needs two states: a transport parameter is either `auto` or a
value. The writing side needs four, because in a PATCH document

* a missing field means "leave this alone",
* `null` means "clear it",
* `"auto"` means "you decide",
* a value means "use this".

Two states cannot express a PATCH. `Param<T>` has four, and the reading side
simply never constructs the two it does not need.

### The transport family is named by the caller

IS-05 transport parameters come in families — RTP, websocket, MQTT, and whatever
a BCP adds next — and **the documents carry no discriminator**. Nothing in a
`transport_params` object says which family it belongs to. The answer is the
Sender's or Receiver's `transport` field, which lives in IS-04, a different API.

So the family cannot be deduced while parsing, and an untagged enum would be
guesswork dressed as a type. Instead the patch types are generic in it:

```rust
ReceiverStagedPatch<P = ReceiverRtpParams>
```

The caller names the family, which it always knows — a Node knows its own
transport, and a controller has just read it off the resource. In exchange it
gets a type that refuses anything belonging to another family, which is how a
Node answers 400 to a controller sending nonsense. RTP is the default because
ST 2110 is what this was written for, not because it is privileged.

Reading stays lenient and writing stays strict, deliberately. A controller
meeting equipment it does not model must still be able to show it, so the client
ignores parameters it has no field for. A Node accepting a patch must not
silently drop half of it, so the patch types deny unknown fields.

`UnknownParams` is the way out of the dilemma: it keeps every key as it arrived
and hands it back unchanged, so a transport this crate has never heard of can
still be read, displayed and returned. Refusing would make this crate the reason
an operator cannot see their device.

### Bare names

`Node`, `Device`, `Receiver` — not `NodeResource`. The crate is the namespace.
A consumer whose own domain already has a `Node` aliases on import, which is one
line at the top of a file rather than a suffix on every type forever.

### Vocabulary

A Sender is **Transmitting** or **Idle**. A Receiver is **Subscribed** or
**Unsubscribed**. Neither is ever "connected": a connection is a relation
between two resources, derived by whoever holds both ends, and not a property
either one reports about itself. The wire says `subscription.active`; that word
is the protocol's, and it means different things on the two resource kinds, so
it does not survive into the type names.

Keeping this straight in the model is what stops a caller from believing a
Sender knows who is listening to it. It does not.

### Two-tier reading

State and transport come from different places and cost different amounts.

A single IS-04 fetch of a Node's resource tree yields the `subscription` of
every Sender and Receiver it has — that is, the whole state of that Node for the
price of one request. Transport detail, the addresses and ports, lives in IS-05
and is fetched per resource.

So a controller learns what is connected to what cheaply, and pays per resource
only for the detail it actually displays. A Node that answers IS-05 slowly
delays its own transport detail and nothing else.

## What deliberately stays out

**Anything product-specific.** A Node that derives its resource identifiers as
UUIDv5 from a vendor's stream names, under a vendor's namespace, is mapping its
own domain onto NMOS. That mapping belongs to the product, not here. The crate
must never learn what a "stream" is.

**Storage.** How a Node holds its resources, versions them and diffs them is a
store concern. This crate describes documents.

**View-models.** Types with `View` in the name, or methods like
`display_name()`, belong to whatever is doing the displaying.

The test of every candidate for inclusion: would a third party implementing an
unrelated NMOS device want this? If the answer needs a paragraph about our
products, the answer is no.

## What is missing, in the order it matters

1. **Sending** a write. The documents are here — `staged`, activation, the
   patches — because both ends read them: a controller composes a patch and a
   Node parses it. What is missing is the client method that puts one on the
   wire, and the bulk endpoint. Those will sit behind a non-default `write`
   feature, so that a consumer which must not write can prove it at compile time
   rather than promise it in a review.

   Describing a write and performing one are different things, and the read-only
   contract is checked against the second: see `tests/read_only.rs`.
2. **Registered mode** — the Registration and Query APIs. In a plant with a
   registry, Nodes register with it and the peer-to-peer mDNS advertisement is
   the fallback for when no registry is found. A controller that only listens
   for `_nmos-node._tcp` sees nothing there. This is a condition of entry, not
   an improvement.
3. **SDP** — `transport_file` is an SDP document, so writing a connection means
   parsing and emitting one.
4. **IS-10 authorization**, then **IS-08 audio channel mapping**.

## How correctness is held

The AMWA JSON Schemas are vendored under `schemas/` and applied as validators in
the test suite, in both directions: what parses must also serialise back into
something the schema accepts. Generated types were rejected for this — the
reasoning is inherited from the controller's ADR-0001.

The schemas prove conformance to the specification. They do not prove
conformance to equipment, which lies in different ways per vendor. That is what
the captured-response corpus under `schemas/examples/` is for, and it is the
part of this repository most worth growing.
