//! # tokio-debouncer
//!
//! A lightweight, cancel-safe async debouncer for [Tokio](https://tokio.rs/) tasks.
//!
//! ## Overview
//!
//! This crate provides a simple, robust, and deterministic debouncer for batching signals or jobs in async workflows.
//! It is especially suited for job queues, event batching, and select-based async workers where you want to coalesce bursts of work and process them efficiently.
//!
//! - Supports both **leading** and **trailing** debounce modes.
//! - Designed for use with `tokio::select!` for robust, cancel-safe batching.
//! - Can be triggered from any thread or task.
//! - Fully tested with simulated time.
//!
//! ## Example
//!
//! ```rust
//! use tokio_debouncer::{Debouncer, DebounceMode};
//! use tokio::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a debouncer with a 100ms cooldown in trailing mode
//!     let debouncer = Debouncer::new(Duration::from_millis(100), DebounceMode::Trailing);
//!     debouncer.trigger(); // Signal an event
//!     let mut guard = debouncer.ready().await; // Wait until ready
//!     // Always call done() as soon as you acquire the guard!
//!     guard.done();
//!     // Do your work after marking as done
//! }
//! ```
//!
//! ## Select-based Job Queue Example
//!
//! ```rust
//! use tokio::{select, time::{sleep, Duration}};
//! use tokio_debouncer::{Debouncer, DebounceMode};
//!
//! #[tokio::main]
//! async fn main() {
//!     let debouncer = Debouncer::new(Duration::from_secs(1), DebounceMode::Trailing);
//!     let debouncer2 = debouncer.clone();
//!     tokio::spawn(async move {
//!         loop {
//!             debouncer2.trigger();
//!             sleep(Duration::from_millis(200)).await;
//!         }
//!     });
//!    let mut iterations = 10;
//!     loop {
//!          iterations -= 1;
//!          if iterations == 0 {
//!              break;
//!          }
//!        // Wait for the debouncer to be ready
//!         select! {
//!             mut guard = debouncer.ready() => {
//!                 guard.done(); // Always call done() first!
//!                 println!("Processing job batch!");
//!             }
//!             _ = sleep(Duration::from_millis(100)) => {
//!                 // Handle other events
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## Best Practice
//!
//! Always call `guard.done()` as soon as you acquire the guard, before doing any actual work. This ensures the debounce state is committed and is cancel-safe. If you do work before calling `done()`, you risk re-processing or double-processing if the task is cancelled or panics.

use std::marker::PhantomData;
use std::sync::{Arc};
use tokio::sync::Notify;
use tokio::time::{Duration, Instant};


// --- parking_lot feature support ---
#[cfg(feature = "parking_lot")]
pub use parking_lot::{Mutex, MutexGuard};
#[cfg(not(feature = "parking_lot"))]
pub use std::sync::{Mutex, MutexGuard};


/// --- MutexExt for poison handling ---
#[cfg(not(feature = "parking_lot"))]
pub trait MutexExt<T> {
    /// Lock the mutex, panicking if poisoned.
    fn risky_lock(&self) -> MutexGuard<T>;
}
#[cfg(not(feature = "parking_lot"))]
impl<T> MutexExt<T> for Mutex<T> {
    fn risky_lock(&self) -> MutexGuard<T> {
        self.lock().expect("Mutex poisoned")
    }
}
#[cfg(feature = "parking_lot")]
pub trait MutexExt<T> {
    /// Lock the parking_lot mutex (never poisoned).
    fn risky_lock(&self) -> MutexGuard<T>;
}
#[cfg(feature = "parking_lot")]
impl<T> MutexExt<T> for Mutex<T> {
    fn risky_lock(&self) -> MutexGuard<T> {
        self.lock()
    }
}

/// The debounce mode: Leading or Trailing.
/// - Leading: fires immediately, then cools down.
/// - Trailing: fires after the last trigger and cooldown (default).
#[derive(Debug)]
pub enum DebounceMode {
    Leading,
    Trailing,
}

/// Internal state for the debouncer.
struct DebouncerState {
    has_run: bool,
    last_run: Instant,
    triggered: bool,
}

/// Shared inner struct for Debouncer.
struct DebouncerInner {
    mode: DebounceMode,
    notifier: Notify,
    cooldown: Duration,
    state: Mutex<DebouncerState>,
}

impl DebouncerInner {
    /// Finalize the debounce state after work is done or dropped.
    fn finalize(&self, pending: bool) {
        let mut state = self.state.risky_lock();
        if state.triggered {
            state.has_run = true;
            state.triggered = pending;
            state.last_run = tokio::time::Instant::now();
            self.notifier.notify_one();
        }
    }
}

/// Guard returned by Debouncer::ready().
/// You must call done() or drop it to finish the debounce.
#[must_use = "DebouncerGuard must be held or .done() must be called to avoid re-triggering"]
pub struct DebouncerGuard<'a> {
    inner: Arc<DebouncerInner>,
    completed: bool,
    _not_send: PhantomData<*const ()>,
    _not_static: PhantomData<&'a ()>,
}

impl<'a> DebouncerGuard<'a> {
    fn new(inner: Arc<DebouncerInner>) -> Self {
        Self {
            inner,
            completed: false,
            _not_send: PhantomData,
            _not_static: PhantomData,
        }
    }

    /// Mark the debounce as done. Always call this as soon as you acquire the guard!
    pub fn done(&mut self) {
        if self.completed {
            return;
        }
        self.completed = true;
        self.inner.finalize(false)
    }
}

impl<'a> Drop for DebouncerGuard<'a> {
    /// If dropped without calling done(), the debounce is finalized as incomplete (re-arms).
    fn drop(&mut self) {
        if !self.completed {
            let inner = self.inner.clone();
            self.completed = true;
            inner.finalize(true);
        }
    }
}

/// Debouncer struct for batching events or jobs.
/// Can be cloned and shared between tasks.
#[derive(Clone)]
pub struct Debouncer {
    inner: Arc<DebouncerInner>,
}

impl Debouncer {
    /// Create a new Debouncer with a cooldown time and mode (Leading or Trailing).
    /// Cooldown is the minimum time between triggers.
    pub fn new(cooldown: Duration, mode: DebounceMode) -> Self {
        let inner = Arc::new(DebouncerInner {
            notifier: Notify::new(),
            cooldown,
            state: Mutex::new(DebouncerState {
                has_run: if matches!(mode, DebounceMode::Leading) {
                    false
                } else {
                    true
                },
                last_run: tokio::time::Instant::now(),
                triggered: false,
            }),
            mode,
        });
        Self { inner }
    }

    /// Check if the debouncer is currently triggered (for diagnostics/testing).
    pub async fn is_triggered(&self) -> bool {
        let state = self.inner.state.risky_lock();
        state.triggered
    }

    /// Trigger the debouncer. Can be called from any thread or task.
    /// Notifies the worker if not already pending.
    pub fn trigger(&self) {
        {
            let mut guard = self.inner.state.risky_lock();
            if matches!(self.inner.mode, DebounceMode::Trailing) {
                guard.last_run = tokio::time::Instant::now();
            }
            if guard.triggered {
                // Already pending, just update the value
                return;
            }
            guard.triggered = true;
        } // guard dropped here
        self.inner.notifier.notify_one();
    }

    /// Wait until the debouncer is ready to run.
    /// Returns a guard that must be used or dropped.
    ///
    /// # Cancel Safety
    /// This method is cancel-safe and does not change internal state until the guard is used.
    #[must_use = "You must await and use the DebouncerGuard to finalize the debounce"]
    pub async fn ready<'a>(&self) -> DebouncerGuard<'a> {
        // Do not change state here to keep it cancel-safe for use inside select
        loop {
            let notified = self.inner.notifier.notified();
            {
                let state = self.inner.state.risky_lock();
                if !state.triggered {
                    drop(state);
                    notified.await;
                    continue;
                }
                let now = tokio::time::Instant::now();
                let next_allowed = state.last_run + self.inner.cooldown;
                match self.inner.mode {
                    DebounceMode::Leading => {
                        if !state.has_run || now >= next_allowed {
                            break;
                        } else {
                            drop(state);
                            tokio::time::sleep_until(next_allowed).await;
                        }
                    }
                    DebounceMode::Trailing => {
                        if now >= next_allowed {
                            break;
                        } else {
                            drop(state);
                            tokio::time::sleep_until(next_allowed).await;
                        }
                    }
                }
            }
        }
        DebouncerGuard::new(self.inner.clone())
    }
}
