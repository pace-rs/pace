//! For tests of pace and of store backends: a settable clock, and the lock
//! that the test doubles share.

use std::sync::{Mutex, MutexGuard};

use jiff::ZonedDateTime;
use pace_error::{ErrorKind, PaceError, PaceResult};

use crate::Clock;

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
