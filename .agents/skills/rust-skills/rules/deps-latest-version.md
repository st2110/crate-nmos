# deps-latest-version

> Use the latest stable version when adding a new crate

## Why It Matters

Newer stable versions usually contain bug fixes, performance improvements, and better compatibility with the rest of the ecosystem. Pinning an old version without a reason leaves the project with known issues and makes future upgrades harder.

## Rules of Thumb

- Prefer the **latest stable** release available on [crates.io](https://crates.io) when adding a crate.
- Use a **flexible version requirement** that allows compatible updates:
  - `1.0` for crates at major version `>= 1` (means `^1.0`, i.e. `>=1.0.0, <2.0.0`).
  - `0.10` for `0.x` crates (means `>=0.10.0, <0.11.0`).
- Avoid exact pins like `=1.2.3` unless a specific bug or policy requires it.
- In a workspace, add the crate to `[workspace.dependencies]` and inherit it in member crates.

## Good

```toml
[dependencies]
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

## Bad

```toml
[dependencies]
# Pinned to an old patch without justification
thiserror = "=1.0.14"

# Too narrow: forces duplicate versions in the tree
serde = "=1.0.210"
```

## Exceptions

- A crate has a known regression in the latest version.
- The project maintains an MSRV that the latest version violates.
- The latest version introduces an API change that cannot be adopted yet.

In these cases, document the reason in a comment next to the version.

## See Also

- [deps-version-flexibility](deps-version-flexibility.md) - Prefer flexible requirements to reduce dependency duplication
- [deps-respect-lock](deps-respect-lock.md) - Prefer the version already recorded in Cargo.lock
- [proj-msrv-declare](proj-msrv-declare.md) - Declare and test the MSRV
