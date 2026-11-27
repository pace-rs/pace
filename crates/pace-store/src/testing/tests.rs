//! The scratch directory lies in memory, and the memory store refuses to
//! read storage that a panicked thread left.

use std::{
    io,
    path::Path,
    sync::{Arc, Mutex},
};

use jiff::ZonedDateTime;
use pace_core::{DeviceId, testing::TestClock};
use pace_time::TimeRange;

use crate::{
    EventStore,
    testing::{MemoryStore, scratch_dir},
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
fn scratch_dir_lies_in_memory_where_the_system_has_it() -> io::Result<()> {
    let dir = scratch_dir()?;
    let memory = Path::new("/dev/shm");
    assert_eq!(dir.path().starts_with(memory), memory.is_dir());
    Ok(())
}
