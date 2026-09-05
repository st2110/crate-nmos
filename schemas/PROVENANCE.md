# Vendored AMWA schemas

These files are copied byte-for-byte from the AMWA specification repositories.
They are the contract this project is checked against: the test suite validates
every resource the controller parses, and everything it serializes, against
them. See `DESIGN.md`, "How correctness is held", for why they are validators
rather than a source of generated types.

Nothing here is edited. A change to the contract is a re-vendor at a new tag,
recorded by updating this file.

## AMWA IS-04 — Discovery and Registration

| | |
|---|---|
| Repository | <https://github.com/AMWA-TV/is-04> |
| Tag | `v1.3.3` |
| Commit | `8e6876d9067cc56f9eca5345d44e41d9e1754444` |
| Vendored on | 2026-09-04 |

- `is-04/v1.3/` — the complete contents of `APIs/schemas/`.
- `examples/is-04/` — the `examples/nodeapi-*.json` documents. The Registration
  and Query API examples are absent because there is no client for those APIs
  yet; they come with registered mode. See `DESIGN.md`, "What is missing".

## AMWA IS-05 — Device Connection Management

| | |
|---|---|
| Repository | <https://github.com/AMWA-TV/is-05> |
| Tag | `v1.1.2` |
| Commit | `325dc5c7d99716c58caa6c00cee4d69cede0e65c` |
| Vendored on | 2026-09-04 |

- `is-05/v1.1/` — the complete contents of `APIs/schemas/`.
- `examples/is-05/` — the `active` responses for Senders and Receivers, and the
  `single` root. The `stage` and `bulk` examples are absent because nothing here
  writes to a device yet; they come with the `write` feature.

## Licence

Both repositories are licensed under the Apache License 2.0, reproduced in
`LICENSE` alongside the upstream `NOTICE` (identical in both repositories). That
is the licence of this crate as well, so there is nothing to reconcile — but the
copyright here is AMWA's rather than ours, and the `NOTICE` is theirs to carry.

These files ship inside the published crate along with the tests that apply
them, so the `NOTICE` above travels with every copy, as Apache-2.0 requires.
