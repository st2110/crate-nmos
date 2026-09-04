# api-function-design

> Keep functions focused, shallow, and easy to call

## Why It Matters

A function with a single responsibility and a small surface area is easier to read, test, and reuse. Deep nesting hides the happy path, and long parameter lists force callers to remember positional meaning and encourage passing `None`/`false` placeholders. Returning early and grouping related parameters keeps the call site obvious.

## Return Early

Return as soon as you know the answer to reduce indentation and keep the happy path left-aligned.

```rust
// Bad: deep nesting hides the normal flow
fn process(request: &Request) -> Result<Response, Error> {
    if let Some(user) = &request.user {
        if user.is_active {
            if let Some(data) = fetch_data(user) {
                Ok(transform(data))
            } else {
                Err(Error::NoData)
            }
        } else {
            Err(Error::InactiveUser)
        }
    } else {
        Err(Error::MissingUser)
    }
}

// Good: guard clauses and early returns
fn process(request: &Request) -> Result<Response, Error> {
    let user = request.user.as_ref().ok_or(Error::MissingUser)?;
    if !user.is_active {
        return Err(Error::InactiveUser);
    }
    let data = fetch_data(user).ok_or(Error::NoData)?;
    Ok(transform(data))
}
```

## Limit Parameters

Aim for **five or fewer** positional parameters. Beyond that, callers struggle to remember the order and meaning of each argument.

```rust
// Bad: seven positional parameters, error-prone call sites
pub fn create_user(
    name: &str,
    email: &str,
    age: u32,
    country: &str,
    newsletter: bool,
    referrer: Option<&str>,
    tags: Vec<String>,
) -> User { ... }

// Call site is a guessing game
let user = create_user("Ann", "ann@example.com", 30, "US", true, None, vec![]);
```

## Group Related Parameters

When a function needs many values, introduce a dedicated `Options`, `Config`, or `Params` struct. Named fields are self-documenting and let callers set only what matters.

```rust
// Good: related values grouped into a struct
pub struct CreateUserOptions<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub age: u32,
    pub country: &'a str,
    pub newsletter: bool,
    pub referrer: Option<&'a str>,
    pub tags: Vec<String>,
}

pub fn create_user(options: CreateUserOptions<'_>) -> User {
    // ...
}

// Call site is readable and order-independent
let user = create_user(CreateUserOptions {
    name: "Ann",
    email: "ann@example.com",
    age: 30,
    country: "US",
    newsletter: true,
    referrer: None,
    tags: vec![],
});
```

For functions with one or two optional values, a builder may be overkill; for several optional values with a required result, use the [builder pattern](api-builder-pattern.md).

## When More Parameters Are OK

A small number of tightly related primitive values can stay positional:

```rust
// Three coordinates are natural as positional args
fn move_to(x: f64, y: f64, z: f64) { ... }
```

## See Also

- [api-builder-pattern](./api-builder-pattern.md) - Builder pattern for complex construction
- [pat-let-else](./pat-let-else.md) - Early-return pattern extraction
- [proj-single-responsibility](./proj-single-responsibility.md) - One responsibility per function
