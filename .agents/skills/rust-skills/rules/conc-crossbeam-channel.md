# conc-crossbeam-channel

> Use `crossbeam-channel` for synchronous multi-producer multi-consumer message passing

## Why It Matters

`crossbeam-channel` provides bounded and unbounded MPMC channels for communication between OS threads. Unlike `std::sync::mpsc`, both `Sender` and `Receiver` are cloneable and shareable, `select!` works over multiple channels, and the crate is optimized for throughput and latency. It is the right choice when you are doing **synchronous** thread-to-thread messaging rather than async task messaging.

## Bad

```rust
use std::sync::mpsc;

// std::sync::mpsc is single-consumer and less flexible.
let (tx, rx) = mpsc::channel();

// Cannot clone the receiver; cannot select over multiple receivers easily.
```

```rust
use crossbeam_channel::unbounded;

// Unbounded channel with a fast producer and slow consumer.
let (tx, rx) = unbounded();
std::thread::spawn(move || {
    loop {
        tx.send(generate_work()).unwrap(); // Memory grows without limit.
    }
});
```

```rust
use crossbeam_channel::{bounded, select};

let (tx, rx) = bounded::<i32>(10);

// WRONG: trying to select inside an async function without blocking the executor.
async fn bad_select(r: crossbeam_channel::Receiver<i32>) {
    select! {
        recv(r) -> msg => println!("{msg:?}"),
    }
}
```

## Good

```rust
use crossbeam_channel::bounded;
use std::thread;

let (tx, rx) = bounded::<i32>(100);

thread::spawn(move || {
    for i in 0..10 {
        tx.send(i).unwrap();
    }
    // Dropping the sender disconnects the channel.
});

for msg in rx {
    println!("{msg}");
}
```

```rust
use crossbeam_channel::{bounded, select, tick, Receiver};
use std::time::Duration;
use std::thread;

// Worker loop with shutdown signal and periodic tick.
fn worker_loop(tasks: Receiver<Task>, shutdown: Receiver<()>) {
    let heartbeat = tick(Duration::from_secs(1));

    loop {
        select! {
            recv(tasks) -> task => match task {
                Ok(task) => process(task),
                Err(_) => break, // all senders dropped
            },
            recv(shutdown) -> _ => break,
            recv(heartbeat) -> _ => emit_heartbeat(),
        }
    }
}
```

## Channel Types

| Type | Use case | Risk |
|------|----------|------|
| `bounded(n)` | Backpressure between producer and consumer | `send` blocks when full |
| `unbounded()` | Bursty traffic where memory is not a concern | Can OOM if producer outpaces consumer |
| `bounded(0)` | Rendezvous: send and receive must meet | Strict handshake, no buffering |
| `after(d)` | Single timeout message | Use inside `select!` |
| `tick(d)` | Periodic timer messages | Use inside `select!` |

## Common Mistakes

1. **Unbounded growth.** An `unbounded()` channel with a fast producer and slow consumer will eventually exhaust memory. Prefer `bounded()` with a sensible capacity unless you can prove the consumer keeps up.
2. **Blocking inside async code.** `crossbeam-channel` is a synchronous primitive; blocking `recv`/`send` inside an async task starves the executor. Use `tokio::sync::mpsc` for async code.
3. **Forgetting to drop senders.** `rx.iter()` and `recv()` return `Err(RecvError)` only after all senders are dropped. Keep track of clones so the receiver knows when to stop.
4. **Unnecessary `unwrap()`.** `send` and `recv` can fail with `SendError` / `RecvError` when the channel disconnects. Handle errors instead of panicking.
5. **`select!` fairness.** When multiple operations are ready, `select!` picks one at random. If you need deterministic priority, use `select_biased!`.

## When to Prefer `tokio::sync::mpsc`

Use `tokio::sync::mpsc` when:
- The sender and receiver live in the same async runtime.
- You need `.await` semantics and cancellation.
- The channel crosses `.await` points.

Use `crossbeam-channel` when:
- Communication happens between OS threads, not async tasks.
- You need `select!` over multiple channels.
- You want the lowest latency synchronous channel.

## When to Add This Crate

See [deps-crate-policy](deps-crate-policy.md) for when to add or use the crates recommended here.

## See Also

- [async-mpsc-queue](async-mpsc-queue.md) - async MPSC channels with `tokio::sync::mpsc`
- [async-bounded-channel](async-bounded-channel.md) - backpressure in async channels
- [conc-send-sync-bounds](conc-send-sync-bounds.md) - thread-safety bounds for spawned code
