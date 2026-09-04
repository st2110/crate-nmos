# doc-comment-audience

> Comment Rust-specific nuances only where they are non-obvious

## Why It Matters

Code should be self-explanatory for *what* it does. Add a comment only when the *why* is Rust-specific and not obvious from the code itself: ownership forces a clone, a lifetime restricts a borrow, an explicit `drop()` controls timing, or a `Send`/`Sync` decision affects threading. Avoid annotating obvious steps.

## Bad

```rust
// Increment the counter
counter += 1;
```

```rust
// Clone the string
let name = name.clone();
```

## Good

```rust
// Clone because the caller still needs `name` after this function returns.
let name = name.clone();
```

```rust
// `Rc` is not `Send`, so we need `Arc` to share this across tasks.
let shared: Arc<Data> = Arc::new(data);
```

## When to Comment

| Situation | Comment?
|-----------|----------|
| The code already says what it does | No |
| A `clone()` is required because of ownership | Yes |
| A lifetime limits a borrow unexpectedly | Yes |
| An explicit `drop()` controls resource timing | Yes |
| A `Send`/`Sync` choice affects threading | Yes |
| An `unsafe` block relies on an invariant | Yes (with `# Safety`) |

## See Also

- [doc-safety-section](./doc-safety-section.md) - Document safety invariants for `unsafe` code
- [own-borrow-over-clone](./own-borrow-over-clone.md) - Prefer borrowing instead of cloning
