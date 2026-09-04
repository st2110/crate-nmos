# deps-version-flexibility

> Prefer flexible version requirements to reduce dependency duplication

## Why It Matters

Cargo resolves the dependency graph by finding versions that satisfy every requirement. The narrower a requirement is, the harder it is for Cargo to pick one version shared by multiple crates. Two dependencies that ask for `tokio = "=1.38.0"` and `tokio = "=1.39.0"` cannot be unified; the build ends up compiling two copies of `tokio`, increasing compile time and binary size.

## Rules of Thumb

- Use the **widest requirement that is still correct** for your code.
- For stable crates (major >= 1), prefer `"1"` or `"1.0"` over `"1.39"` or `"=1.39.0"`.
- For `0.x` crates, the second number is the breaking-change boundary, so `"0.10"` is the flexible equivalent.
- Only pin exactly (`=x.y.z`) when a specific version is required for correctness or policy.

## How Cargo Interprets Common Forms

| Requirement | Accepted range | Use when |
|-------------|----------------|----------|
| `"1"` | `>=1.0.0, <2.0.0` | Default for stable crates |
| `"1.0"` | `>=1.0.0, <2.0.0` | Same as `"1"`, more explicit |
| `"1.39"` | `>=1.39.0, <2.0.0` | You need an API introduced in 1.39 |
| `"=1.39.0"` | Exactly 1.39.0 | Specific bug or policy requires it |
| `"0.10"` | `>=0.10.0, <0.11.0` | Default for `0.x` crates |

## Good

```toml
[dependencies]
serde = "1"
tokio = "1"
bytes = "1"
```

## Bad

```toml
[dependencies]
# Narrows the range for no reason
serde = "1.0.210"

# Exact pin without justification
tokio = "=1.39.0"
```

## Exceptions

- You depend on an API or fix introduced in a specific minor/patch version.
- A transitive dependency has a known-bad version that must be excluded.
- The project policy requires exact pins for reproducibility.

In these cases, document the reason next to the requirement.

## See Also

- [deps-latest-version](deps-latest-version.md) - Use the latest stable version when adding a crate
- [deps-respect-lock](deps-respect-lock.md) - Prefer the version already recorded in Cargo.lock
- [proj-workspace-deps](proj-workspace-deps.md) - Use workspace dependency inheritance for consistent versions
