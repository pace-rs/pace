//! Events from all devices are merged in the order of their hybrid logical
//! clocks.

use pace_core::Hlc;

use crate::TestResult;

#[test]
fn a_later_wall_time_comes_later_whatever_the_counter() -> TestResult {
    let earlier = Hlc::new("2026-11-26T08:02:11.204Z".parse()?, 7);
    let later = Hlc::new("2026-11-26T08:02:11.205Z".parse()?, 0);

    assert!(earlier < later);
    Ok(())
}

#[test]
fn the_counter_orders_events_with_the_same_wall_time() -> TestResult {
    let wall = "2026-11-26T08:02:11.204Z".parse()?;

    assert!(Hlc::new(wall, 0) < Hlc::new(wall, 1));
    Ok(())
}
