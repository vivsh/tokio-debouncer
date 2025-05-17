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
* ✅ Simple API with minimal overhead
* ✅ Fully tested using `tokio::time::pause` for time-based simulation

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
    let mut guard = debouncer.ready().await; // Wait until ready
    // Do work after debounce
    guard.done(); // Always call done() as soon as you acquire the guard!
}
```

---

## 🥪 API Overview

* `Debouncer::new(Duration, DebounceMode)` — create a new debouncer
* `Debouncer::trigger()` — signal that an event occurred
* `Debouncer::ready()` — await until it's appropriate to run
* `DebouncerGuard::done()` — mark the work as complete

> **Note:**
> - `ready()` is cancel-safe and does not change internal state.
> - Only `guard.done()` commits state changes.
> - If `.done()` is not called, the debouncer assumes the work was incomplete and re-arms itself.
> - **Always call `guard.done()` as soon as you acquire the guard, before doing any actual work.**

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
            mut guard = debouncer.ready() => {
                // Always call guard.done() as early as possible!
                guard.done();
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
- **Call `guard.done()` immediately after acquiring the guard to commit the debounce state.**

---

> **Best Practice:**
> Always call `guard.done()` as soon as you acquire the guard, before doing any actual work. This ensures the debounce state is committed and is cancel-safe. If you do work before calling `done()`, you risk re-processing or double-processing if the task is cancelled or panics.

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
