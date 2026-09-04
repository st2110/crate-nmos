# proj-free-functions-vs-methods

> Prefer methods over free functions that operate on a single struct's fields *(Recommended)*

## Why It Matters

When a function only makes sense together with one struct and mainly reads or writes that struct's fields, making it an associated method keeps the API cohesive. Methods are discovered through IDE autocomplete on the type, live next to the data they modify, and make ownership and borrowing explicit via `&self` / `&mut self`. A free function floating in another module forces the reader to hunt for it and obscures the natural owner of the behavior.

This rule is **recommended**, not absolute. Free functions are still the right choice when the operation involves several equally-important types or when the function is a pure conversion that does not logically belong to either side.

## Bad

```rust
pub struct ConnectionPool {
    connections: Vec<Connection>,
    max_size: usize,
}

// This function is useless without ConnectionPool and reaches into its fields.
pub fn add_connection(pool: &mut ConnectionPool, conn: Connection) {
    if pool.connections.len() < pool.max_size {
        pool.connections.push(conn);
    }
}
```

## Good

```rust
pub struct ConnectionPool {
    connections: Vec<Connection>,
    max_size: usize,
}

impl ConnectionPool {
    pub fn add(&mut self, conn: Connection) {
        if self.connections.len() < self.max_size {
            self.connections.push(conn);
        }
    }
}
```

## When Free Functions Are Fine

```rust
// Neither type is clearly the owner: both are peers.
pub fn connect(source: &Endpoint, target: &Endpoint) -> Channel {
    // ...
}

// Pure conversion that does not need private fields.
pub fn json_to_config(json: &str) -> Result<Config, ParseError> {
    // ...
}
```

## See Also

- [proj-thin-lib-rs](./proj-thin-lib-rs.md) - Keep `lib.rs` a thin public facade
- [api-extension-trait](./api-extension-trait.md) - Add methods to external types with extension traits
