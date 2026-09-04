# proj-no-pointless-reexport

> Don't create re-export-only modules for a single struct

## Why It Matters

A module that exists only to `pub use` one type adds nesting without adding value. It forces users to remember an extra path segment and creates files whose only purpose is indirection. If a struct already exposes a clear public API, expose it directly from the crate root or from the module where it naturally lives. Re-export modules make sense when they aggregate several related items or hide a deep internal layout, not when they wrap a single type.

## Bad

```rust
// src/lib.rs
pub mod user;

// src/user/mod.rs — only re-exports
pub use crate::internal::User;

// Users write:
use my_crate::user::User;  // Unnecessary `user` segment
```

## Good

```rust
// src/lib.rs
mod internal;

pub use internal::User;

// Users write:
use my_crate::User;
```

## When Re-export Modules Are Fine

```rust
// Aggregating several related public items behind a stable facade.
pub mod transport {
    pub use crate::internal::http::HttpClient;
    pub use crate::internal::grpc::GrpcClient;
    pub use crate::internal::ws::WebSocketClient;
}

// Hiding a deep, refactor-prone internal layout.
pub mod parser {
    pub use crate::parsing::lexer::Lexer;
    pub use crate::parsing::tree::AstBuilder;
}
```

## See Also

- [proj-pub-use-reexport](./proj-pub-use-reexport.md) - Use `pub use` for a clean public API
- [proj-thin-lib-rs](./proj-thin-lib-rs.md) - Keep `lib.rs` a thin public facade
