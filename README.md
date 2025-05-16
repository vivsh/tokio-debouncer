# tokio-debouncer

[![Crates.io](https://img.shields.io/crates/v/tokio-debouncer.svg)](https://crates.io/crates/tokio-debouncer)
[![Docs.rs](https://docs.rs/tokio-debouncer/badge.svg)](https://docs.rs/tokio-debouncer)
[![License](https://img.shields.io/crates/l/tokio-debouncer.svg)](https://crates.io/crates/tokio-debouncer)

---

**`tokio-debouncer`** is a lightweight, cancel-safe async debouncer for [Tokio](https://tokio.rs/) tasks.
It provides precise control over event handling using **leading** and **trailing** debounce modes, with deterministic state transitions.

---

## 🚀 Features

* ✅ Asynchronous debounce for event-driven tasks
* ✅ Leading and trailing debounce strategies
* ✅ Deterministic, cancel-safe state transitions
* ✅ Simple API with minimal overhead
* ✅ Fully tested using `tokio::time::pause` for time-based simulation

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
    let debouncer = Debouncer::new(Duration::from_millis(100), DebounceMode::Trailing);

    debouncer.trigger().await;

    let mut guard = debouncer.ready().await;

    // Do work after debounce
    guard.done().await;
}
```

---

## 🥪 API Overview

* `Debouncer::new(Duration, DebounceMode)` — create a new debouncer
* `Debouncer::trigger()` — signal that an event occurred
* `Debouncer::ready()` — await until it's appropriate to run
* `DebouncerGuard::done()` — mark the work as complete

> **Note:**
> `ready()` is cancel-safe and does not change internal state.
> Only `guard.done()` commits state changes.
> If `.done()` is not called, the debouncer assumes the work was incomplete and re-arms itself.

---

## 🔄 Debounce Modes

| Mode     | Behavior                                                       |
| -------- | -------------------------------------------------------------- |
| Leading  | Fires **immediately**, then cools down                         |
| Trailing | Waits for cooldown period to elapse after the **last trigger** |

---

## ⚡ Cancel-Safe Example with `tokio::select!`

You can safely await `debouncer.ready()` inside a `tokio::select!` block.
If the branch is canceled, no state is mutated and the debounce remains valid:

```rust
use tokio::{select, time::{sleep, Duration}};
use tokio_debouncer::{Debouncer, DebounceMode};

#[tokio::main]
async fn main() {
    let debouncer = Debouncer::new(Duration::from_secs(1), DebounceMode::Trailing);

    loop {
        debouncer.trigger().await;

        select! {
            mut guard = debouncer.ready() => {
                // Do the work
                guard.done().await;
            }
            _ = sleep(Duration::from_millis(100)) => {
                // Cancelled or skipped this round
                // debounce state remains unchanged
            }
        }
    }
}
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
