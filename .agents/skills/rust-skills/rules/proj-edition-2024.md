# proj-edition-2024

> Use Rust 2024 edition for new projects and migrations

## Why It Matters

The Rust 2024 edition brings language improvements that reduce boilerplate, clarify unsafe code, and fix long-standing papercuts: `if let` chains, disjoint closure captures by default, mandatory `unsafe {}` blocks inside `unsafe fn`, `unsafe extern` blocks, `#[unsafe(no_mangle)]`, and the MSRV-aware resolver (`resolver = "3"`). Staying on an old edition means new code misses these improvements and accumulates migration debt. For greenfield projects there is no reason to start on 2021; for existing projects, edition migration should be planned as routine toolchain hygiene.

## Rules of Thumb

- Start every **new project or crate** on `edition = "2024"`.
- Set `resolver = "3"` explicitly in workspace roots; it is the default for the 2024 edition and enables MSRV-aware dependency resolution.
- When migrating an existing crate, run `cargo fix --edition` and review every mechanical change before committing.
- Keep `rust-version` (MSRV) in sync with the edition: 2024 edition requires Rust 1.85.0 or later.

## Bad

```toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"
# Missing rust-version and using an old edition for a new project
```

## Good

```toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[workspace]
resolver = "3"
```

## Migration Checklist

1. Bump `edition` to `"2024"` in `Cargo.toml`.
2. Add or update `rust-version` to at least `"1.85"`.
3. Set workspace `resolver = "3"` if you use a workspace.
4. Run `cargo fix --edition` to apply mechanical changes.
5. Review unsafe-related rewrites (`extern` blocks, `no_mangle`, `link_section`).
6. Run the full test suite and clippy before merging.

## See Also

- [proj-msrv-declare](./proj-msrv-declare.md) - Declare and test the MSRV
- [proj-workspace-deps](./proj-workspace-deps.md) - Workspace dependency inheritance
- [pat-if-let-chains](./pat-if-let-chains.md) - `if let` chains require the 2024 edition
- [unsafe-extern-block](./unsafe-extern-block.md) - `unsafe extern` blocks in 2024
- [unsafe-no-mangle-unsafe](./unsafe-no-mangle-unsafe.md) - `#[unsafe(no_mangle)]` in 2024
