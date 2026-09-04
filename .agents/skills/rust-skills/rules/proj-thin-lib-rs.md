# proj-thin-lib-rs

> Keep `lib.rs` a thin public facade

## Why It Matters

`lib.rs` is the front door of a crate. When it is cluttered with implementations, data types, and helpers, readers cannot tell at a glance what the crate exposes. A thin facade declares modules, re-exports the public API with `pub use`, and leaves the actual code in focused sibling module files. This makes the crate's contract explicit and keeps the root file easy to scan.

## Bad

```rust
// src/lib.rs
pub mod parser;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Config {
    values: HashMap<String, String>,
}

impl Config {
    pub fn new() -> Self {
        Self { values: HashMap::new() }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    pub fn set(&mut self, key: String, value: String) {
        self.values.insert(key, value);
    }
}

pub fn load_config(path: &std::path::Path) -> std::io::Result<Config> {
    // ...
}
```

## Good

```rust
// src/lib.rs
mod config;
mod loader;

pub use config::Config;
pub use loader::load_config;
```

```rust
// src/config.rs
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Config {
    values: HashMap<String, String>,
}

impl Config {
    pub fn new() -> Self {
        Self { values: HashMap::new() }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    pub fn set(&mut self, key: String, value: String) {
        self.values.insert(key, value);
    }
}
```

```rust
// src/loader.rs
use std::path::Path;
use crate::Config;

pub fn load_config(path: &Path) -> std::io::Result<Config> {
    // ...
}
```

## Guidelines

- Use `mod` for private implementation modules.
- Use `pub mod` only when the submodule itself is part of the public API.
- Use `pub use` to lift the items users actually need up to the crate root.
- Put unit tests in `#[cfg(test)] mod tests` inside the module they test, or in a sibling `..._tests.rs` / `tests/` file per project convention.

## See Also

- [proj-pub-use-reexport](./proj-pub-use-reexport.md) - Use `pub use` for a clean public API
- [proj-free-functions-vs-methods](./proj-free-functions-vs-methods.md) - Prefer methods for struct-bound behavior
