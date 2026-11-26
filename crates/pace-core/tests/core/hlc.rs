//! The hybrid logical clock: every device orders events the same way, and a
//! device's own order survives its clock jumping back.

use jiff::Timestamp;
use pace_core::Hlc;
use proptest::prelude::*;

use crate::TestResult;

fn timestamp(millis: i64) -> Result<Timestamp, TestCaseError> {
    Timestamp::from_millisecond(millis).map_err(|error| TestCaseError::fail(error.to_string()))
}

// 2001 to 2065, in milliseconds.
const RANGE: std::ops::Range<i64> = 1_000_000_000_000..3_000_000_000_000;

#[test]
fn the_first_tick_is_the_wall_time_in_milliseconds() -> TestResult {
    let now: Timestamp = "2026-11-26T08:02:11.204937Z".parse()?;

    assert_eq!(
        Hlc::tick(None, now),
        Hlc::new("2026-11-26T08:02:11.204Z".parse()?, 0)
    );
    Ok(())
}

#[test]
fn tick_in_the_same_millisecond_counts_up() -> TestResult {
    let now: Timestamp = "2026-11-26T08:02:11.204Z".parse()?;
    let first = Hlc::tick(None, now);

    assert_eq!(Hlc::tick(Some(first), now), Hlc::new(now, 1));
    Ok(())
}

proptest! {
    #[test]
    fn tick_comes_after_the_last(last_wall in RANGE, counter in 0u32..1000, now in RANGE) {
        let last = Hlc::new(timestamp(last_wall)?, counter);
        let next = Hlc::tick(Some(last), timestamp(now)?);

        prop_assert!(next > last);
        prop_assert!(next.wall() >= timestamp(now)?);
    }

    #[test]
    fn ticks_increase_even_when_the_clock_jumps_back(walls in prop::collection::vec(RANGE, 1..50)) {
        let mut last = None;
        for wall in walls {
            let next = Hlc::tick(last, timestamp(wall)?);
            if let Some(last) = last {
                prop_assert!(next > last);
            }
            last = Some(next);
        }
    }
}
