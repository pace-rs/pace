//! Behaviour of `pace_time`.
//!
//! # Ported from the old pace
//!
//! The old `crates/time` (chrono) had 60 tests, plus one commented out. The
//! ones that tested behaviour are ported here with literal expectations and
//! a fixed "now". The ones that tested the chrono wrapper types (`PaceDate`,
//! `PaceTime`, `PaceDuration`: `Deref`, `From`, `Eq`, arithmetic) are dropped,
//! because jiff's types replace them.
//!
//! | Old test | Now |
//! | -------- | --- |
//! | `time_frame`: `get_time_frame_from_flags_today` | `frames::frames_are_named_as_on_the_command_line` |
//! | `time_range`: `convert_…_today`, `…_yesterday`, `…_current_week`, `…_last_week`, `…_current_month`, `…_last_month`, `…_current_year`, `…_last_year` | `frames::frames_cover_whole_periods`, plus the Monday, Sunday, New Year and January cases the old code got wrong |
//! | `time_range`: `convert_…_date_range` | `frames::from_and_to_include_both_days` |
//! | `time_range`: `convert_…_specific_date` | `frames::a_single_day_ends_at_the_next_midnight` |
//! | `time_range`: `is_in_range_options_passes` | `frames::a_range_contains_its_start_but_not_its_end` |
//! | `time_range`: `is_in_range_options_fails` | `frames::a_range_that_ends_before_it_starts_is_rejected`, `frames::from_after_to_is_rejected` |
//! | `date`: `default` (today is the local date) | `frames::frames_use_the_zone_of_now` |
//! | `date`: `display` | `frames::days_between_read_as_a_sentence` |
//! | `date`: `from_str_passes`, `is_future_is_false`; `date_time`: `begin_date_time_new` | `at::a_date_and_time_is_taken_as_is_in_the_zone_of_now` |
//! | `date`: `from_str_fails` | `at::text_that_is_no_time_explains_the_grammar` |
//! | `date`: `is_future_is_true`; `date_time`: `is_future_fails` | `at::a_time_in_the_future_is_rejected` |
//! | `duration`: `calculate_duration_with_time_zone` (3 cases) | `durations::elapsed_time_does_not_depend_on_the_offsets` |
//! | `duration`: `calculate_duration_passes` | `durations::elapsed_time_counts_seconds` |
//! | `duration`: `calculate_duration_fails` | `durations::an_end_before_the_start_is_rejected` |
//! | `duration`: `from_str_passes` (seconds) | `durations::durations_are_typed_with_units`; a bare number now asks for a unit: `durations::a_bare_number_asks_for_a_unit` |
//! | `duration`: `from_str_fails` | `durations::text_that_is_no_duration_is_rejected` |
//! | `date_time`: `begin_date_time_default` (now in whole seconds) | the system clock in the app: `clock::tests::the_system_clock_shows_whole_seconds_in_its_zone` |
//! | `duration`: `duration_to_str` ("just now") | `display::moments_read_relative_to_now` |
//! | `date`: `try_from_ymd_passes`, `try_from_fails` | dropped: the same parsing as `from_str` |
//! | `date`: `with_start`, `from_pace_date_time`, `from_pace_date_time_ref`, `deref`, `eq`, `ne` | dropped: wrapper type |
//! | `time`: all 6 (`from_pace_date_time`, `…_ref`, `from_naive_time`, `deref`, `eq`, `ord`) | dropped: wrapper type |
//! | `duration`: `from_duration`, `from_chrono_duration`, `try_from`, `default`, `zero`, `from_seconds`, `add`, `add_assign`, `sub`, `sub_assign`, `sub_assign_with_u64`, `sub_assign_below_zero`, `as_secs`, `as_duration`, `as_minutes`, `as_hours`, `as_days`, `as_weeks` | dropped: wrapper type; `SignedDuration` does this |

mod at;
mod display;
mod durations;
mod frames;

type TestResult = Result<(), Box<dyn std::error::Error>>;
