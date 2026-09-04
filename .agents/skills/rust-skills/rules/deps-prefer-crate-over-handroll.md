# deps-prefer-crate-over-handroll

> Prefer a small, low-overhead crate over hand-rolling when it significantly reduces new code at optimal performance

## Why It Matters

Hand-rolled implementations of non-trivial logic create code that must be written, reviewed, tested, audited, and maintained forever. A small, focused, well-tested crate that solves the exact problem with minimal overhead is usually cheaper and safer than a custom version.

This rule is the complement to [anti-premature-optimize](anti-premature-optimize.md): do not optimize before profiling, but also do not reinvent wheels that already exist as reliable crates.

## When to Prefer a Crate

Prefer an external crate when all of the following are true:

- The problem is non-trivial (parsing, serialization, hashing, concurrency primitives, numeric utilities, date/time handling, etc.).
- The crate is small and focused, not a large framework that pulls in dozens of transitive dependencies.
- Its runtime and compile-time overhead are acceptable for the use case.
- It is actively maintained and widely used enough to trust.
- Using it removes a meaningful amount of new code you would otherwise have to write and maintain.

## When to Hand-Roll Instead

Write your own implementation when:

- The logic is trivial (a few lines) and a crate would add more integration overhead than value.
- The crate has a heavy dependency tree, long compile times, or unacceptable binary-size impact.
- The crate's license is incompatible with the project.
- The crate's API is a poor fit and wrapping it would not reduce complexity.

## Examples

```rust
// GOOD: use a small, focused crate for a well-understood problem
use sha2::{Sha256, Digest};

fn hash_password(password: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password);
    hasher.update(salt);
    format!("{:x}", hasher.finalize())
}
```

```rust
// BAD: hand-rolling a parser for a standard format when `nom` or `winnow`
// would produce a shorter, more reliable implementation
fn parse_ipv4_manual(input: &str) -> Option<(u8, u8, u8, u8)> {
    // many lines of fragile string splitting and validation
    todo!()
}
```

## See Also

- [deps-crate-policy](deps-crate-policy.md) - Check existing crate usage before adding a new dependency
- [anti-premature-optimize](anti-premature-optimize.md) - Do not optimize before profiling
