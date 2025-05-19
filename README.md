# tokio-debouncer

[![Crates.io](https://img.shields.io/crates/v/tokio-debouncer.svg)](https://crates.io/crates/tokio-debouncer)
[![Docs.rs](https://docs.rs/tokio-debouncer/badge.svg)](https://docs.rs/tokio-debouncer)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)
---

**`tokio-debouncer`** is a lightweight, cancel-safe async debouncer for [Tokio](https://tokio.rs/) tasks.

---

## 🚀 Features

* ✅ Asynchronous debounce for event-driven tasks
* ✅ Leading and trailing debounce strategies
* ✅ Deterministic, cancel-safe state transitions
* ✅ Simple, ergonomic API 
* ✅ Fully tested using `tokio::time::pause` for time-based simulation
* ✅ Feature-flagged `parking_lot` support (enabled by default)

---

## 🎯 Best Suited For: Job Queues and Select-Based Workflows

This crate is especially designed for scenarios where you need to debounce jobs or events in a background worker, and the debounce logic must be integrated with a `tokio::select!` loop. This is common in job queues, event batching, and async pipelines where you want to coalesce bursts of work and process them efficiently.

**Why use tokio-debouncer for job queues?**
- Await `debouncer.ready()` inside a `tokio::select!` block to respond to multiple signals (timers, shutdown, new jobs) without missing or double-processing events.
- Cancel-safe: if your select branch is cancelled, the debounce state is not mutated, so you never lose a job or process too early.
- Trigger the debouncer from any thread or task; the worker will pick up the work at the right time.

---

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
tokio-debouncer = "<latest-version>"
```

---

## 🛠️ Usage

```rust
use tokio_debouncer::{Debouncer, DebounceMode};
use tokio::time::Duration;

#[tokio::main]
async fn main() {
    // Create a debouncer with a 100ms cooldown in trailing mode
    let debouncer = Debouncer::new(Duration::from_millis(100), DebounceMode::Trailing);
    debouncer.trigger(); // Signal an event
    let _guard = debouncer.ready().await; // Wait until ready; debounce is finalized on drop
    // Do your work here
} // guard dropped here, debounce is finalized
```

---

## 🥪 API Overview

* `Debouncer::new(Duration, DebounceMode)` — create a new debouncer
* `Debouncer::trigger()` — signal that an event occurred
* `Debouncer::ready()` — await until it's appropriate to run

> **Note:**
> - `ready()` is cancel-safe and does not change internal state.
> - The debounce state is finalized automatically when the guard is dropped. You do not need to call any method to commit the debounce; simply let the guard go out of scope after acquiring it. This ensures robust, cancellation-safe batching, even if your task is cancelled or panics after acquiring the guard.

---

## 🔄 Debounce Modes

| Mode     | Behavior                                                       |
| -------- | -------------------------------------------------------------- |
| Leading  | Fires **immediately**, then cools down                         |
| Trailing | Waits for cooldown period to elapse after the **last trigger** |

---

## ⚡ Select-Based Job Queue Example (Recommended Pattern)

This is the most robust and idiomatic way to use `tokio-debouncer` in a job queue or event-driven worker:

```rust
use tokio::{select, time::{sleep, Duration}};
use tokio_debouncer::{Debouncer, DebounceMode};

#[tokio::main]
async fn main() {
    // Create a debouncer for batching jobs
    let debouncer = Debouncer::new(Duration::from_secs(1), DebounceMode::Trailing);

    // Simulate jobs arriving from another task
    let debouncer2 = debouncer.clone();
    tokio::spawn(async move {
        loop {
            debouncer2.trigger(); // Simulate job arrival
            sleep(Duration::from_millis(200)).await;
        }
    });

    loop {
        select! {
            _ = debouncer.ready() => {
                // Now process your batch of jobs
                println!("Processing job batch!");
            }
            _ = sleep(Duration::from_millis(100)) => {
                // Handle other events, shutdown, etc.
            }
        }
    }
}
```

- `debouncer.trigger()` can be called from any thread or task to signal new work.
- The worker loop uses `select!` to wait for either debounce readiness or other events.

---

### Best Practice

The debounce state is now finalized automatically when the guard is dropped. You do not need to call any method to commit the debounce; simply let the guard go out of scope after acquiring it. This ensures robust, cancellation-safe batching, even if your task is cancelled or panics after acquiring the guard.

---

## ⚙️ Cargo Features

- **`parking_lot`** *(default)*: Use `parking_lot::Mutex` for improved performance and poisoning behavior. Disable with `default-features = false` to use `std::sync::Mutex` instead.
- **`std`**: (Always enabled) Use standard library features. Present for compatibility with some dependency managers.

Example disabling `parking_lot`:

```toml
[dependencies]
tokio-debouncer = { version = "0.3", default-features = false }
```

---

## 🥪 Testing

The crate includes comprehensive tests using `tokio::time::pause` and `advance` to simulate time.

```sh
cargo test
```

---

## 🦠 Minimum Supported Rust Version (MSRV)

* Rust 1.60+

---

## 📄 License

Licensed under the MIT License.
See [LICENSE](LICENSE) for details.
