# proj-import-organization

> Keep imports explicit and ordered

## Why It Matters

Wildcard imports hide where names come from, which makes code harder to read and easier to break when dependencies add new items. A consistent import order lets reviewers scan a file quickly and reduces merge conflicts. `rustfmt` enforces the order, but the rule still belongs in review because it communicates intent.

## No Wildcard Imports

Avoid `use module::*;` except in preludes, `use super::*;` in test modules, and explicit prelude re-exports.

```rust
// Bad: unclear where HashMap, Error, and Client come from
use std::collections::*;
use my_crate::network::*;
use serde::*;

fn run() -> Result<(), Error> {
    let map = HashMap::new();
    let client = Client::new();
    ...
}
```

```rust
// Good: every imported name is explicit
use std::collections::HashMap;

use my_crate::network::Client;
use serde::{Deserialize, Serialize};

fn run() -> Result<(), my_crate::Error> {
    let map = HashMap::new();
    let client = Client::new();
    ...
}
```

## Import Order

Group imports in three blocks separated by blank lines:

1. `std` / `core` / `alloc`
2. External crates
3. Local modules / crate internals

```rust
use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::info;

use crate::config::Config;
use crate::error::Error;
```

`rustfmt` reorders items within each group automatically. If you disable `rustfmt` for a file, preserve this order manually.

## Self-Imports and Re-exports

Keep `pub use` re-exports in the module where the public API is assembled (usually `lib.rs` or a dedicated `prelude`). Do not mix internal `use crate::...` items with public re-exports in the same block.

```rust
// Inside lib.rs or a facade module
pub use crate::client::Client;
pub use crate::error::Error;
```

## See Also

- [proj-thin-lib-rs](./proj-thin-lib-rs.md) - Keep lib.rs a thin public facade
- [proj-prelude-module](./proj-prelude-module.md) - Create prelude modules for common imports
