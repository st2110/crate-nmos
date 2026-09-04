# async-task-owner-lifetime

> Long-lived async tasks must die with their owner

## Why It Matters

Spawning an async task that holds a strong reference to its owner creates a reference cycle: the owner keeps the task handle (or JoinSet), and the task keeps the owner alive via `Arc`. The owner never drops, so its resources — temporary directories, sockets, child handles — leak for the lifetime of the process. In tests this is especially harmful because hundreds of short-lived owners can accumulate background loops.

## Use a Weak Reference or Cancellation Token

Pass the spawned task a `Weak` pointer or a `CancellationToken` rather than a cloned `Arc` of the owner state. The task loop exits as soon as the owner is gone.

```rust
use std::sync::Weak;
use tokio::sync::Mutex;

pub struct Poller {
    state: Arc<Mutex<PollerState>>,
    handle: Option<JoinHandle<()>>,
}

impl Poller {
    pub fn spawn(inner: Arc<Mutex<PollerState>>) -> Self {
        let weak = Arc::downgrade(&inner);

        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;

                let Some(state) = weak.upgrade() else {
                    // Owner dropped; stop the loop and release everything.
                    break;
                };

                let mut guard = state.lock().await;
                guard.poll().await;
            }
        });

        Self {
            state: inner,
            handle: Some(handle),
        }
    }
}
```

With a `CancellationToken`:

```rust
use tokio_util::sync::CancellationToken;

pub struct Poller {
    cancel: CancellationToken,
    handle: Option<JoinHandle<()>>,
}

impl Poller {
    pub fn spawn() -> Self {
        let cancel = CancellationToken::new();
        let child = cancel.child_token();

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = child.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        // do work
                    }
                }
            }
        });

        Self {
            cancel,
            handle: Some(handle),
        }
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
```

## Verify with a Drop Test

The task loop must exit when the owner is dropped. Prove it with a test, not just reasoning.

```rust
#[tokio::test]
async fn poller_stops_when_dropped() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    let poller = Poller::spawn(move || async move {
        tx.send(1).await.ok();
    });

    // Ensure the task is running.
    assert_eq!(rx.recv().await, Some(1));

    drop(poller);

    // Give the runtime a chance to finish the task.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // The channel should be closed because the sender lived inside the task.
    assert!(rx.recv().await.is_none());
}
```

## Exceptions

Tasks that are intentionally process-scoped — listeners, composition-root daemons — are exempt. Document that ownership explicitly in a comment so the lifetime is intentional, not an accidental clone.

## See Also

- [async-cancellation-token](./async-cancellation-token.md) - Cooperative cancellation
- [async-clone-before-await](./async-clone-before-await.md) - Clone data before await points
- [own-arc-shared](./own-arc-shared.md) - Shared ownership with Arc
