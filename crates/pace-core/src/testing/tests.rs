//! The test doubles refuse to read a value that a panicked thread left.

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex},
};

use jiff::ZonedDateTime;
use pace_time::TimeRange;

use crate::{
    Clock, DeviceId, EventStore,
    testing::{MemoryStore, TestClock},
};

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
fn memory_store_refuses_poisoned_storage() -> TestResult {
    let clock = Arc::new(TestClock::new(nine()?));
    let store = MemoryStore::new(
        "0193a8d0-000d-7000-8000-000000000001".parse::<DeviceId>()?,
        clock,
    );
    poison(&store.storage);

    let error = store
        .read(&TimeRange::new(None, None)?)
        .err()
        .ok_or("the store read poisoned storage")?;

    assert_eq!(error.to_string(), "the memory store's lock is poisoned");
    Ok(())
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
