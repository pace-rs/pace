//! The hybrid logical clock: every device orders events the same way, and a
//! device's own order survives its clock jumping back.

use jiff::Timestamp;
use pace_core::Hlc;
use proptest::{prelude::*, strategy::ValueTree as _, test_runner::TestRunner};

use crate::{TestResult, fixed_seed};

fn timestamp(millis: i64) -> Result<Timestamp, TestCaseError> {
    Timestamp::from_millisecond(millis).map_err(|error| TestCaseError::fail(error.to_string()))
}

// 2001 to 2065, in milliseconds.
const RANGE: std::ops::Range<i64> = 1_000_000_000_000..3_000_000_000_000;

#[test]
fn hlc_with_later_wall_time_comes_later_whatever_its_counter() -> TestResult {
    let earlier = Hlc::new("2026-11-26T08:02:11.204Z".parse()?, 7);
    let later = Hlc::new("2026-11-26T08:02:11.205Z".parse()?, 0);

    assert!(earlier < later);
    Ok(())
}

#[test]
fn hlc_counter_orders_readings_of_same_wall_time() -> TestResult {
    let wall = "2026-11-26T08:02:11.204Z".parse()?;

    assert!(Hlc::new(wall, 0) < Hlc::new(wall, 1));
    Ok(())
}

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

#[test]
fn tick_with_full_counter_moves_to_next_millisecond() -> TestResult {
    let now: Timestamp = "2026-11-26T08:02:11.204Z".parse()?;
    let full = Hlc::new(now, u32::MAX);

    assert_eq!(
        Hlc::tick(Some(full), now),
        Hlc::new("2026-11-26T08:02:11.205Z".parse()?, 0)
    );
    Ok(())
}

#[test]
fn property_tests_draw_same_cases_on_every_run() -> TestResult {
    let draw = || -> Result<Vec<u64>, String> {
        let mut runner = TestRunner::new(fixed_seed());
        (0..5)
            .map(|_| {
                let tree = any::<u64>().new_tree(&mut runner);
                tree.map(|tree| tree.current())
                    .map_err(|reason| reason.to_string())
            })
            .collect()
    };
    assert_eq!(draw()?, draw()?);
    Ok(())
}

proptest! {
    #![proptest_config(fixed_seed())]

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
