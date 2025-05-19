// tests/debounce.rs

use tokio_debouncer::Debouncer;
use tokio_debouncer::DebounceMode;
use tokio::time::{self, Duration};


#[tokio::test(start_paused = true)]
async fn leading_runs_immediately_on_first_trigger() {
    // Test: Leading mode should yield immediately on first trigger
    let debounce = Debouncer::new(Duration::from_secs(10), DebounceMode::Leading);
    debounce.trigger();

    let _guard = debounce.ready().await;
    assert!(debounce.is_triggered().await); // should still be triggered until guard is dropped
}

#[tokio::test(start_paused = true)]
async fn leading_respects_cooldown() {
    // Test: Leading mode should only yield again after cooldown has passed
    let debounce = Debouncer::new(Duration::from_secs(10), DebounceMode::Leading);

    debounce.trigger();
    debounce.ready().await; // guard is dropped automatically

    debounce.trigger();
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

    debounce.trigger();
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

    debounce.trigger();
    time::advance(Duration::from_secs(3)).await;
    debounce.trigger(); // should extend the debounce

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
    // Test: Dropping the guard clears the trigger flag
    let debounce = Debouncer::new(Duration::from_secs(10), DebounceMode::Leading);
    debounce.trigger();
    {
        let _guard = debounce.ready().await;
        // guard dropped here
    }
    assert!(!debounce.is_triggered().await);
}


#[tokio::test(start_paused = true)]
async fn multiple_triggers_yield_only_once() {
    // Test: Multiple triggers don't cause multiple yields within cooldown
    let debounce = Debouncer::new(Duration::from_secs(5), DebounceMode::Trailing);

    debounce.trigger();
    debounce.trigger();
    debounce.trigger();

    time::advance(Duration::from_secs(5)).await;
    debounce.ready().await;

    let mut yielded = false;
    tokio::select! {
        _ = debounce.ready() => { yielded = true; }
        _ = time::sleep(Duration::from_secs(1)) => {}
    }
    assert!(!yielded, "No second yield without new trigger");
}


