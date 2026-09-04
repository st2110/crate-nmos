# deps-respect-lock

> Prefer the version already recorded in Cargo.lock

## Why It Matters

`Cargo.lock` is the resolved snapshot of the dependency tree. If a crate already appears there, the project has been built and tested with that exact version. Reusing that version keeps the tree stable and avoids introducing duplicate copies of the same crate.

## Rules of Thumb

- Before adding a new crate, check whether it (or a compatible version) is already in `Cargo.lock`.
- If it is already there, declare a version requirement that matches the locked version (e.g. the same major.minor range).
- Do not bump a crate version as a side effect of adding a new feature unless the bump is required.
- When you intentionally upgrade, run `cargo update -p <crate>` and run the test suite.

## How to Check

```bash
# Exact crate and version in the lockfile
grep -A1 'name = "<crate-name>"' Cargo.lock

# Or with cargo
cargo tree -p <crate-name>
```

## Good

```toml
# Cargo.lock already has serde 1.0.210
[dependencies]
serde = "1.0"
```

```bash
# Intentional upgrade, not accidental
cargo update -p serde
cargo test
```

## Bad

```toml
# Forces a newer patch than Cargo.lock, potentially creating a duplicate
tokio = "=1.39.0"
# when Cargo.lock already has tokio 1.38.0 used by another dependency
```

## Relationship with Latest Version

This rule is optional and complements [deps-latest-version](deps-latest-version.md). In a greenfield project, prefer latest. In an existing project, prefer the locked version unless there is a reason to upgrade.

## See Also

- [deps-latest-version](deps-latest-version.md) - Use the latest stable version when adding a crate
- [deps-version-flexibility](deps-version-flexibility.md) - Prefer flexible requirements to reduce dependency duplication
- [proj-workspace-deps](proj-workspace-deps.md) - Use workspace dependency inheritance for consistent versions
