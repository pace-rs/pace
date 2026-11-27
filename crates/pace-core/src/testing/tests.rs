//! The test clock refuses to read a time that a panicked thread left.

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Mutex,
};

use jiff::ZonedDateTime;

use crate::{Clock, testing::TestClock};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn nine() -> Result<ZonedDateTime, jiff::Error> {
    "2026-11-26T09:00[Europe/Berlin]".parse()
}

/// Panic in another thread while it holds `lock`.
fn poison<T: Send>(lock: &Mutex<T>) {
    std::thread::scope(|scope| {
        _ = scope
            .spawn(|| {
                let _guard = lock.lock();
                panic!("a test thread panics while it holds the lock");
            })
            .join();
    });
}

#[test]
fn test_clock_refuses_poisoned_time() -> TestResult {
    let clock = TestClock::new(nine()?);
    poison(&clock.0);

    let read = catch_unwind(AssertUnwindSafe(|| clock.now()));

    let panic = read.err().ok_or("the clock read a poisoned time")?;
    assert_eq!(
        panic.downcast_ref::<String>().map(String::as_str),
        Some("the test clock's lock is poisoned")
    );
    Ok(())
}

#[test]
fn clock_behind_trait_shows_its_type_and_time_in_debug() -> TestResult {
    let clock = TestClock::new(nine()?);
    let clock: &dyn Clock = &clock;

    let shown = format!("{clock:?}");

    assert!(shown.starts_with("TestClock("), "{shown}");
    assert!(
        shown.contains("2026-11-26T09:00:00+01:00[Europe/Berlin]"),
        "{shown}"
    );
    Ok(())
}
