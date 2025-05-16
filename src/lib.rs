use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio::time::{Duration, Instant};

/// The debounce mode: Leading or Trailing.
/// Leading runs at the start, Trailing at the end.
#[derive(Debug)]
pub enum DebounceMode {
    Leading,
    Trailing,
}

struct DebouncerState {
    has_run: bool,
    last_run: Instant,
    triggered: bool,
}

struct DebouncerInner {
    mode: DebounceMode,
    notifier: Notify,
    cooldown: Duration,
    state: Mutex<DebouncerState>,
}

impl DebouncerInner {
    async fn finalize(&self, pending: bool) {
        let mut state = self.state.lock().await;
        if state.triggered {
            state.has_run = true;
            state.triggered = pending;
            state.last_run = tokio::time::Instant::now();
            self.notifier.notify_one();
        }
    }
}

/// This guard is returned by Debouncer::ready().
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

    /// Call this to mark the debounce as done.
    /// After calling, the guard is completed.
    pub async fn done(&mut self) {
        if self.completed {
            return;
        }
        self.completed = true;
        self.inner.finalize(false).await
    }
}

impl<'a> Drop for DebouncerGuard<'a> {
    fn drop(&mut self) {
        if !self.completed {
            let inner = self.inner.clone();
            self.completed = true;
            tokio::spawn(async move {
                inner.finalize(true).await;
            });
        }
    }
}

/// This struct is used to debounce events.
/// It can be cloned and shared between tasks.
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

    pub async fn is_triggered(&self) -> bool {
        let state = self.inner.state.lock().await;
        state.triggered
    }


    /// Call this when you want to trigger the debouncer.
    /// It will notify if not already pending.
    pub async fn trigger(&self) {
        let mut guard = self.inner.state.lock().await;
        
        if matches!(self.inner.mode, DebounceMode::Trailing) {
            guard.last_run = tokio::time::Instant::now();
        }        
        
        if guard.triggered {
            // Already pending, just update the value
            return;
        }

        guard.triggered = true;
        drop(guard);
        self.inner.notifier.notify_one();
    }

    /// Wait until the debouncer is ready to run.
    /// Returns a guard that must be used or dropped.
    #[must_use = "You must await and use the DebouncerGuard to finalize the debounce"]
    pub async fn ready<'a>(&self) -> DebouncerGuard<'a> {
        // Donot change state here to keep it cancel-safe for use inside select
        loop {
            let notified = self.inner.notifier.notified();
            {
                let state = self.inner.state.lock().await;

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

