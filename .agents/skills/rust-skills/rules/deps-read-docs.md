# deps-read-docs

> Read the crate documentation for the version declared in Cargo.toml

## Why It Matters

Crate APIs evolve. A method, trait, or macro that exists in the latest version may not exist in the version pinned by the project, and vice versa. Reading the docs for the exact version prevents copy-pasting outdated or too-new code.

## How to Read the Right Version

For a crate `foo` at version `1.2.3`:

- docs.rs: `https://docs.rs/foo/1.2.3/foo/`
- crates.io page links to the exact version docs.
- `cargo doc --open -p foo` builds docs for the locally resolved version.

## Good

```toml
[dependencies]
tokio = "1"
```

```rust
use tokio::time::{sleep, Duration};

// Verified against tokio 1.x docs for the exact resolved version.
async fn wait() {
    sleep(Duration::from_millis(100)).await;
}
```

## Bad

```rust
// Copied from tokio 0.2 docs; does not compile on tokio 1.x
use tokio::time::delay_for;

async fn wait() {
    delay_for(Duration::from_millis(100)).await; // ERROR: unresolved function
}
```

## Practical Habit

Before writing code against a crate you do not use daily:

1. Open `Cargo.toml` and note the declared version (or the version in `Cargo.lock`).
2. Read the docs for that exact version.
3. Check the crate's changelog / migration guide for breaking changes if you are upgrading.

## See Also

- [deps-latest-version](deps-latest-version.md) - Use the latest stable version when adding a crate
- [deps-respect-lock](deps-respect-lock.md) - Prefer the version already recorded in Cargo.lock
