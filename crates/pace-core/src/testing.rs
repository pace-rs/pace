//! For tests of pace and of store backends: a settable clock, deterministic
//! IDs, and the lock that the test doubles share.

use std::sync::{
    Mutex, MutexGuard,
    atomic::{AtomicU64, Ordering},
};

use jiff::ZonedDateTime;
use pace_error::{ErrorKind, PaceError, PaceResult};
use uuid::{Builder, Uuid};

use crate::{Clock, Ids};

#[cfg(test)]
mod tests;

/// A clock that tests set.
///
/// A clock whose lock a panicked thread held panics when it is read or
/// set, with the error of [`locked`]: [`Clock::now`] cannot return an
/// error.
#[derive(Debug)]
pub struct TestClock(Mutex<ZonedDateTime>);

impl TestClock {
    /// A clock that shows `now`.
    #[must_use]
    pub const fn new(now: ZonedDateTime) -> Self {
        Self(Mutex::new(now))
    }

    /// Move the clock to `now`, also backwards.
    ///
    /// # Panics
    ///
    /// If a thread panicked while it held the clock's lock.
    pub fn set(&self, now: ZonedDateTime) {
        *self.time() = now;
    }

    fn time(&self) -> MutexGuard<'_, ZonedDateTime> {
        match locked(&self.0, "test clock") {
            Ok(time) => time,
            Err(error) => panic!("{error}"),
        }
    }
}

impl Clock for TestClock {
    fn now(&self) -> ZonedDateTime {
        self.time().clone()
    }
}

/// The value behind `lock`, or an error that names the `what` it guards.
///
/// A thread that panicked while it held the lock may have left the value
/// half-changed, so the test doubles never read it; `pace_store`'s
/// in-memory store guards its storage with it too.
///
/// # Errors
///
/// If a thread panicked while it held the lock.
pub fn locked<'lock, T>(lock: &'lock Mutex<T>, what: &str) -> PaceResult<MutexGuard<'lock, T>> {
    lock.lock().map_err(|_| {
        PaceError::new(ErrorKind::Internal, "the {what}'s lock is poisoned")
            .attach_context("what", what)
            .ask_report()
    })
}

/// Deterministic IDs: the timestamp of a fixed moment, and where a UUIDv7
/// has random bits, a mix of that moment, a seed and a counter.
///
/// IDs differ in their last characters, like random ones, so short IDs stay
/// short; simulated devices take different seeds.
#[derive(Debug)]
pub struct SequentialIds {
    millis: u64,
    seed: u16,
    counter: AtomicU64,
}

impl SequentialIds {
    /// IDs with the timestamp of `moment` and the given seed.
    #[must_use]
    pub fn starting_at(moment: &ZonedDateTime, seed: u16) -> Self {
        Self {
            millis: u64::try_from(moment.timestamp().as_millisecond()).unwrap_or_default(),
            seed,
            counter: AtomicU64::new(0),
        }
    }
}

/// A well-mixed, deterministic 64-bit value (the `SplitMix64` finalizer).
const fn mix(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

impl Ids for SequentialIds {
    fn next_id(&self) -> Uuid {
        let count = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
        let mixed = mix(self.millis ^ (u64::from(self.seed) << 48) ^ count);
        let mut bytes = [0; 10];
        bytes[..2].copy_from_slice(&self.seed.to_be_bytes());
        bytes[2..].copy_from_slice(&mixed.to_be_bytes());
        Builder::from_unix_timestamp_millis(self.millis, &bytes).into_uuid()
    }
}
