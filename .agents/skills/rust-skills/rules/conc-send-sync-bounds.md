# conc-send-sync-bounds

> Apply `Send` and `Sync` trait bounds appropriately

## Why It Matters

`Send` and `Sync` are the two auto-traits that Rust uses to decide what can cross thread boundaries. Adding bounds that are too strict makes APIs hard to use and compose; bounds that are too loose let unsound code compile. In async code every `.await` is a potential thread hop, so future-capturing types usually need `Send`. Choosing the right bound keeps code both safe and ergonomic.

## Bad

```rust
// Overly strict: requires T: Send even though the function never sends T to another thread
fn count_items<T: Send>(items: &[T]) -> usize {
    items.len()
}
```

```rust
// Missing bound: this future may be sent between tokio worker threads, so the handle must be Send
async fn process<T>(item: T) {
    tokio::spawn(async move {
        work(item).await;
    });
}
// error[E0277]: `T` cannot be sent between threads safely
```

```rust
// Holding a !Send guard across an await point prevents the future from being Send
async fn bad(shared: Arc<Mutex<Data>>) {
    let guard = shared.lock().unwrap();
    some_async().await; // guard is held across await
    drop(guard);
}
```

## Good

```rust
// Only require the bounds the implementation actually needs
fn count_items<T>(items: &[T]) -> usize {
    items.len()
}
```

```rust
use tokio::task::JoinHandle;

// Spawned futures need Send + 'static, so ask for the bound on the public API
async fn process<T>(item: T) -> JoinHandle<()>
where
    T: Send + 'static,
{
    tokio::spawn(async move {
        work(item).await;
    })
}
```

```rust
use std::sync::{Arc, Mutex};

// Keep critical sections short and do not hold locks across await points
async fn good(shared: Arc<Mutex<Data>>) {
    let snapshot = {
        let guard = shared.lock().unwrap();
        guard.clone_state()
    };
    some_async(snapshot).await;
}
```

## Decision Guide

| Context | Required bound | Notes |
|---------|---------------|-------|
| `std::thread::spawn` | `F: Send + 'static` | The closure and its captures move to a new thread |
| `tokio::spawn` | `F: Send + 'static`, `F::Output: Send` | Futures may resume on a different worker thread after each `.await` |
| `tokio::task::spawn_blocking` | `F: Send + 'static`, `F::Output: Send` | Closure runs on a blocking thread pool |
| Shared read-only state | `T: Send + Sync` | `Arc<T>` is `Send + Sync` iff `T: Send + Sync` |
| Single-threaded async (`current_thread`) | `Send` not required by the runtime, but still useful for tests/reuse | Prefer `Send` when the type is meant to be general purpose |

## Key Points

- Prefer `Send` on captured data in async functions that will be spawned; it is the default expectation in a multi-threaded runtime.
- `Sync` is needed when the same value is accessed concurrently through shared references (`&T`). `Arc<T>` is only `Sync` when `T: Sync`.
- Holding a synchronous lock (`Mutex`, `RwLock`) across an `.await` point makes the future `!Send` and can cause deadlocks in async executors. Drop the guard before awaiting.
- Do not add `Send`/`Sync` bounds to generic functions unless the body requires them. Unnecessary bounds leak implementation details into the API.
- For types that are not thread-safe by default, decide explicitly: wrap them in `Arc<Mutex<T>>`/`Arc<RwLock<T>>`, or mark them as single-threaded and keep them off the multi-threaded runtime.
- When manually implementing `Send` or `Sync`, see [unsafe-send-sync-manual](unsafe-send-sync-manual.md) and document the safety invariant with `// SAFETY:`.

## See Also

- [unsafe-send-sync-manual](unsafe-send-sync-manual.md) - document manual `Send`/`Sync` implementations
- [own-arc-shared](own-arc-shared.md) - use `Arc<T>` for thread-safe shared ownership
- [own-mutex-interior](own-mutex-interior.md) - use `Mutex<T>` for interior mutability across threads
- [async-no-lock-await](async-no-lock-await.md) - avoid holding locks across await points
- [async-tokio-runtime](async-tokio-runtime.md) - configure Tokio for your workload
