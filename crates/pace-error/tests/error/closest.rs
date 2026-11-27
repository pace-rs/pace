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
fn best_score_wins_wherever_it_stands() {
    assert_eq!(
        closest_name("today", ["today", "todays"].into_iter()),
        Some("today")
    );
    assert_eq!(
        closest_name("today", ["todays", "today"].into_iter()),
        Some("today")
    );
}

#[test]
fn name_must_score_above_threshold() {
    // Jaro-Winkler scores 0.822 for "client" and 0.797 for "clients".
    assert_eq!(
        closest_name("clxxnt", ["client"].into_iter()),
        Some("client")
    );
    assert_eq!(closest_name("clxxnt", ["clients"].into_iter()), None);
}

#[test]
fn tie_names_last_of_equals() {
    assert_eq!(closest_name("abc", ["abd", "abe"].into_iter()), Some("abe"));
}

#[test]
fn no_known_names_names_none() {
    assert_eq!(closest_name("version", std::iter::empty()), None);
}
