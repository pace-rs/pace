//! The contract of [`closest_name`]: which known name a hint suggests.

use pace_error::closest_name;

#[test]
fn typo_names_closest_known_name() {
    assert_eq!(
        closest_name("verison", ["version", "projects"].into_iter()),
        Some("version")
    );
}

#[test]
fn name_far_from_every_known_one_names_none() {
    assert_eq!(
        closest_name("zzz", ["version", "projects"].into_iter()),
        None
    );
}

#[test]
fn best_score_wins_over_order() {
    assert_eq!(
        closest_name("today", ["todays", "today"].into_iter()),
        Some("today")
    );
}

#[test]
fn tie_names_last_of_equals() {
    assert_eq!(closest_name("abc", ["abd", "abe"].into_iter()), Some("abe"));
}

#[test]
fn no_known_names_names_none() {
    assert_eq!(closest_name("version", std::iter::empty()), None);
}
