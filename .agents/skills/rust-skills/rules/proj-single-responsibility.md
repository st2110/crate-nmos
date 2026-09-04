# proj-single-responsibility

> One responsibility per function and per type

## Why It Matters

A function or type that does exactly one thing is easier to name, test, and reason about. When responsibilities pile up, names become vague (`process`, `handle`, `do_stuff`), tests must cover many unrelated paths, and changes in one area risk breaking another.

## Functions

A function should either answer a question, perform one action, or compute one value. If you find yourself joining unrelated steps with "and" when describing what a function does, split it.

```rust
// Bad: parses, validates, saves, and notifies
fn process_user_input(raw: &str) -> Result<(), Error> {
    let parsed: Config = serde_json::from_str(raw)?;
    validate(&parsed)?;
    save(&parsed)?;
    notify(&parsed);
    Ok(())
}
```

```rust
// Good: each function has one responsibility
fn parse_config(raw: &str) -> Result<Config, Error> {
    serde_json::from_str(raw).map_err(Error::from)
}

fn apply_config(config: &Config) -> Result<(), Error> {
    validate(config)?;
    save(config)?;
    notify(config);
    Ok(())
}
```

## Types

A struct or enum should model one concept. Avoid types that are a grab bag of unrelated fields just because they travel together through one function.

```rust
// Bad: mixes user data with request metadata and rendering state
struct UserPageData {
    user_name: String,
    user_email: String,
    request_id: String,
    template_name: String,
    is_cached: bool,
}
```

```rust
// Good: separate concerns
struct User {
    name: String,
    email: String,
}

struct PageContext {
    request_id: String,
    template: String,
    cached: bool,
}
```

## When to Combine

Small, tightly coupled steps that are always invoked together can stay in one function. The goal is not minimal line count but a single, coherent reason to change.

## See Also

- [api-function-design](./api-function-design.md) - Keep functions focused and shallow
- [proj-type-design](./proj-type-design.md) - Composition and encapsulation
