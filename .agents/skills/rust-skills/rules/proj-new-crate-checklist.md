# proj-new-crate-checklist

> Follow this checklist when creating a new crate in the workspace

## Why It Matters

`cargo new` and `cargo init` generate a `Cargo.toml` with `edition = "2021"` by default. If the crate is created without an explicit override, it immediately violates the project-wide edition policy and accumulates migration debt. A short checklist prevents this and keeps new crates consistent with the rest of the workspace.

## Checklist

1. Create the crate from the workspace root:
   ```bash
   cargo new --lib crates/<crate-name>
   # or
   cargo new --bin crates/<crate-name>
   ```
2. **Immediately** open the generated `Cargo.toml` and set:
   - `edition = "2024"`
   - `rust-version = "1.85"` (or inherit from the workspace)
3. Inherit workspace metadata whenever possible:
   ```toml
   [package]
   name = "<crate-name>"
   version.workspace = true
   edition.workspace = true
   rust-version.workspace = true
   ```
4. Add the crate path to the workspace `members` list in the root `Cargo.toml` if it is not already covered by a glob.
5. Run `cargo check -p <crate-name>` (or `make check`) and fix any warnings before committing.

## Bad

```toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"
```

The default `cargo new` output is not acceptable for this codebase.

## Good

```toml
[package]
name = "my-crate"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
```

Or, when workspace inheritance is not available:

```toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
```

## See Also

- [proj-edition-2024](./proj-edition-2024.md) - Use Rust 2024 edition for new projects and migrations
- [proj-workspace-deps](./proj-workspace-deps.md) - Workspace dependency inheritance
- [proj-msrv-declare](./proj-msrv-declare.md) - Declare and test the MSRV
