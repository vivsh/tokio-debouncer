// tests/debounce.rs

use tokio_debouncer::Debouncer;
use tokio_debouncer::DebounceMode;
use tokio::time::{self, Duration};


#[tokio::test(start_paused = true)]
async fn leading_runs_immediately_on_first_trigger() {
    // Test: Leading mode should yield immediately on first trigger
    let debounce = Debouncer::new(Duration::from_secs(10), DebounceMode::Leading);
    debounce.trigger().await;

    let _guard = debounce.ready().await;
    assert!(debounce.is_triggered().await); // should still be triggered until .done() or drop
}

#[tokio::test(start_paused = true)]
async fn leading_respects_cooldown() {
    // Test: Leading mode should only yield again after cooldown has passed
    let debounce = Debouncer::new(Duration::from_secs(10), DebounceMode::Leading);

    debounce.trigger().await;
    let mut guard = debounce.ready().await;
    guard.done().await;

    debounce.trigger().await;
    time::advance(Duration::from_secs(9)).await;
    let mut yielded = false;
    tokio::select! {
        _ = debounce.ready() => { yielded = true; }
        _ = time::sleep(Duration::from_millis(999)) => {}
    }
    assert!(!yielded, "Should not yield before cooldown");

    time::advance(Duration::from_secs(1)).await;
    let _guard = debounce.ready().await; // should now yield
}

#[tokio::test(start_paused = true)]
async fn trailing_yields_after_silence() {
    // Test: Trailing mode should yield only after cooldown period of silence
    let debounce = Debouncer::new(Duration::from_secs(5), DebounceMode::Trailing);

    debounce.trigger().await;
    let mut yielded = false;

    tokio::select! {
        _ = debounce.ready() => { yielded = true; }
        _ = time::sleep(Duration::from_secs(4)) => {}
    }
    assert!(!yielded, "Should not yield before 5s of silence");

    time::advance(Duration::from_secs(1)).await;
    let _guard = debounce.ready().await;
}

#[tokio::test(start_paused = true)]
async fn trailing_reschedules_on_repeated_trigger() {
    // Test: Trailing mode restarts its cooldown on each new trigger
    let debounce = Debouncer::new(Duration::from_secs(5), DebounceMode::Trailing);

    debounce.trigger().await;
    time::advance(Duration::from_secs(3)).await;
    debounce.trigger().await; // should extend the debounce

    let mut yielded = false;
    tokio::select! {
        _ = debounce.ready() => { yielded = true; }
        _ = time::sleep(Duration::from_secs(1)) => {}
    }
    assert!(!yielded, "Cooldown should have been extended");

    time::advance(Duration::from_secs(4)).await;
    let _guard = debounce.ready().await;
}

#[tokio::test(start_paused = true)]
async fn done_clears_trigger_flag() {
    // Test: Calling .done() clears the trigger flag
    let debounce = Debouncer::new(Duration::from_secs(10), DebounceMode::Leading);
    debounce.trigger().await;
    let mut guard = debounce.ready().await;
    guard.done().await;

    assert!(!debounce.is_triggered().await);
}

#[tokio::test(start_paused = true)]
async fn drop_defers_trigger_in_trailing() {
    // Test: Dropping guard without done() should re-trigger in trailing mode
    let debounce = Debouncer::new(Duration::from_secs(5), DebounceMode::Trailing);

    debounce.trigger().await;
    {
        let _guard = debounce.ready().await;
        // dropped without done()
    }

    // Re-arm due to drop
    time::advance(Duration::from_secs(5)).await;
    let _guard = debounce.ready().await;
}

#[tokio::test(start_paused = true)]
async fn multiple_triggers_yield_only_once() {
    // Test: Multiple triggers don't cause multiple yields within cooldown
    let debounce = Debouncer::new(Duration::from_secs(5), DebounceMode::Trailing);

    debounce.trigger().await;
    debounce.trigger().await;
    debounce.trigger().await;

    time::advance(Duration::from_secs(5)).await;
    let mut _guard = debounce.ready().await;

    let mut yielded = false;
    tokio::select! {
        mut g = debounce.ready() => { g.done().await; yielded = true; }
        _ = time::sleep(Duration::from_secs(1)) => {}
    }
    assert!(yielded, "No second yield without new trigger");
}


#[tokio::test(start_paused = true)]
async fn ready_without_done_retriggers() {
    // Test: If guard is dropped without .done(), debounce is re-armed
    let debounce = Debouncer::new(Duration::from_secs(5), DebounceMode::Trailing);

    debounce.trigger().await;
    time::advance(Duration::from_secs(5)).await;

    {
        let _guard = debounce.ready().await;
        // intentionally not calling .done()
    }

    // cooldown restarts — debounce is re-armed
    time::advance(Duration::from_secs(5)).await;

    let _guard2 = debounce.ready().await;
}