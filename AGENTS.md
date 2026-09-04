# AGENTS.md

Guidance for anyone — human or agent — working in this repository.

## What this is

`nmos` is a Rust implementation of AMWA NMOS for SMPTE ST 2110 networks. It is
an open source library with two first-party consumers that pull in opposite
directions: a controller that reads other people's Nodes, and a Node that serves
its own resources. Read `DESIGN.md` before changing a public type — it records
why the shapes are what they are.

## Layout

`CLAUDE.md` is a symlink to this file and `.claude` is a symlink to `.agents` —
edit `AGENTS.md` and `.agents/`, not the symlinks.

`.agents/skills/rust-skills` is vendored and MIT-licensed (by leonardomso),
which this repository's Apache-2.0 does not override; keep its `license:`
frontmatter intact. None of it ships in the published crate — see the `exclude`
list in `Cargo.toml`.

There is deliberately no openspec here. This crate follows a specification
somebody else writes; the design decisions that are ours live in `DESIGN.md`.

## Rules

### Language: English only

This is an open source, international project. Everything written into the
repository — code comments, identifiers, documentation, commit messages, issue
and merge request descriptions, log messages, error strings — MUST be in
English. No exceptions, even when the conversation happens in another language.

### Both consumers, every time

A change to a public type must be considered from both ends: the side that
parses a document it did not write, and the side that emits one. A shape that
serves only one of them is how this crate becomes two crates again.

Nothing product-specific enters the model. The test: would a third party
implementing an unrelated NMOS device want this? If the answer needs a paragraph
about our products, the answer is no.

### Mirrors

The project lives in two mirrors:

- A public one: <https://github.com/st2110/crate-nmos>. This is the only address
  that may be referenced anywhere in the repository.
- An internal corporate one. Its address, hostname, and the very fact of a
  specific internal location MUST NEVER appear in the sources — not in code,
  comments, docs, README, `Cargo.toml`, CI config, commit messages, issue links,
  or example URLs. When a link is needed, use the public mirror.

### Rust

Invoke the `rust-skills` skill before writing, reviewing or refactoring Rust,
including dependency choices and crate structure, and follow its rules.

### Testing

- **Tests come first.** Write the test before the code it tests. A change that
  adds behaviour without a test that fails before it and passes after is not
  finished.
- **The happy path is not coverage.** Exercise the edges too: empty and
  maximum-size input, zero and boundary values, malformed and hostile input.
  Use `proptest` where the input space is large enough that hand-picked cases
  will miss things.
- **Every bug gets a regression test**, written before the fix and never
  weakened afterwards.
- The vendored AMWA schemas are validators, not code generators. What parses
  must serialise back into something the schema accepts.

### No panics in production code

`unwrap()`, `expect()`, `panic!()`, `todo!()`, and slice indexing that can go
out of bounds are forbidden outside tests. This crate drives equipment that is
on air; it must return an error, not abort. Propagate with `?`. The lints in
`Cargo.toml` enforce it; relax them in `#[cfg(test)]` code only.

### Features are additive

Cargo unifies features across the whole graph, so a feature must only ever add
capability. `write`, when it lands, adds the ability to change a device; its
absence is the baseline and is what lets a read-only consumer prove it cannot
write.

### MSRV

`rust-version` is the floor of edition 2024, not the toolchain in
`rust-toolchain.toml`. A library that demands this month's compiler excludes the
people who would otherwise adopt it.

The declared version is verified by hand — `cargo +1.85 check --no-default-features`
and `--all-features` both pass — but a number nobody checks decays. **CI must hold
it**, and until CI exists, check it before every release.

### Commits

- Keep the body short: at most 3 lines. Say what changed and why.
- If the work belongs to a ticket, put a trailing `Refs #<id>` line.
- English only, like everything else here.
