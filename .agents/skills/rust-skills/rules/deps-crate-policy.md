# deps-crate-policy

> Check existing crate usage before adding a new dependency

## Why It Matters

Adding a crate to a project is not a purely local decision. A new dependency affects compile times, binary size, audit surface, and the dependency tree for every downstream crate. The right action depends on whether the project already knows the crate.

## Decision Tree

When a rule or task suggests using an external crate, follow this order:

1. **The crate is already a dependency** (listed in `Cargo.toml` or `Cargo.lock`)
   - Use it freely in new or changed code.
   - Prefer existing patterns in the codebase over inventing new usage styles.

2. **The crate is not yet a dependency**
   - **Existing / mature project:** propose adding it to the user and wait for approval.
   - **New / greenfield project:** add and use it directly, unless the project has an explicit policy against it.

3. **A different crate already solves the same problem**
   - Use the one already present, even if another crate is marginally more popular.
   - Only introduce an alternative if the existing one has a concrete, blocking limitation.

4. **No crate solves the problem yet**
   - For non-trivial functionality, prefer a small, low-overhead crate that significantly reduces new code at optimal performance over hand-rolling. See [deps-prefer-crate-over-handroll](deps-prefer-crate-over-handroll.md).
   - For trivial logic (a few lines), hand-rolling may be simpler than adding a dependency.

## How to Check

```bash
# Is the crate already a dependency?
grep -E '^<crate-name>\s*=' Cargo.toml

# Or search the workspace
rg '<crate-name>\s*=' Cargo.toml Cargo.lock
```

## Examples

```rust
// OK: thiserror is already in Cargo.toml -> use it
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid value for {key}")]
    InvalidValue { key: String },
}
```

```rust
// BAD: silently adding a new crate to an existing project
// Before using `miette`, ask whether the project wants another error library
// alongside thiserror/anyhow.
```

## See Also

- [deps-latest-version](deps-latest-version.md) - Use the latest stable version when adding a crate
- [deps-read-docs](deps-read-docs.md) - Read the crate docs for the version in Cargo.toml
- [deps-respect-lock](deps-respect-lock.md) - Prefer the version already recorded in Cargo.lock
