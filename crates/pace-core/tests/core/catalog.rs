//! The catalog: roles, clients and project records, their keys and rates.

use jiff::SignedDuration;
use pace_core::{
    ArchiveReason, ClientKey, RoleId, RoleKey, RoleKind, Rounding, RoundingMode, RoundingPer,
};
use rstest::rstest;

use crate::TestResult;

#[rstest]
#[case("work")]
#[case("area-health")]
fn role_key_is_lowercase_words_with_dashes(#[case] key: &str) -> TestResult {
    assert_eq!(key.parse::<RoleKey>()?.as_str(), key);
    Ok(())
}

#[rstest]
#[case("acme")]
#[case("acme-gmbh-2")]
fn client_key_is_lowercase_words_with_dashes(#[case] key: &str) -> TestResult {
    assert_eq!(key.parse::<ClientKey>()?.to_string(), key);
    Ok(())
}

#[test]
fn role_key_with_uppercase_is_refused_and_named() {
    let error = "Work".parse::<RoleKey>().err();

    assert_eq!(
        error.map(|error| error.to_string()).as_deref(),
        Some("`Work` is not a role key")
    );
}

#[test]
fn client_key_with_space_is_refused_with_example() {
    let hint = "acme gmbh"
        .parse::<ClientKey>()
        .err()
        .and_then(|error| error.hint().map(ToOwned::to_owned));

    assert_eq!(
        hint.as_deref(),
        Some("a key has lowercase letters, digits and single dashes, like `acme`")
    );
}

#[test]
fn role_id_reads_uuid_v7() -> TestResult {
    let text = "0193a8d0-000d-7000-8000-000000000001";

    assert_eq!(text.parse::<RoleId>()?.to_string(), text);
    Ok(())
}

const fn mins(minutes: i64) -> SignedDuration {
    SignedDuration::from_mins(minutes)
}

fn quarter_hours(mode: RoundingMode) -> Result<Rounding, Box<dyn std::error::Error>> {
    Ok(Rounding::new(RoundingPer::Entry, mins(15), mode)?)
}

#[rstest]
#[case::up(RoundingMode::Up, 46, 60)]
#[case::up_on_step(RoundingMode::Up, 45, 45)]
#[case::down(RoundingMode::Down, 59, 45)]
#[case::nearest_below_half(RoundingMode::Nearest, 52, 45)]
#[case::nearest_below_next_half(RoundingMode::Nearest, 67, 60)]
#[case::nearest_above_half(RoundingMode::Nearest, 53, 60)]
fn rounding_to_quarter_hours(
    #[case] mode: RoundingMode,
    #[case] worked: i64,
    #[case] rounded: i64,
) -> TestResult {
    assert_eq!(quarter_hours(mode)?.round(mins(worked)), mins(rounded));
    Ok(())
}

#[test]
fn half_step_rounds_up_to_nearest() -> TestResult {
    let worked = mins(7) + SignedDuration::from_secs(30);

    assert_eq!(
        quarter_hours(RoundingMode::Nearest)?.round(worked),
        mins(15)
    );
    Ok(())
}

#[test]
fn no_rounding_is_default_and_keeps_duration() {
    let worked = SignedDuration::from_secs(1234);

    assert_eq!(Rounding::default(), Rounding::NONE);
    assert_eq!(Rounding::NONE.round(worked), worked);
    assert_eq!(Rounding::NONE.per(), None);
}

#[test]
fn rounding_keeps_its_rule() -> TestResult {
    let rounding = Rounding::new(RoundingPer::Day, mins(6), RoundingMode::Down)?;

    assert_eq!(
        (rounding.per(), rounding.step(), rounding.mode()),
        (
            Some(RoundingPer::Day),
            Some(mins(6)),
            Some(RoundingMode::Down)
        )
    );
    Ok(())
}

#[rstest]
#[case::zero(SignedDuration::ZERO)]
#[case::negative(mins(-15))]
fn rounding_step_must_be_positive(#[case] step: SignedDuration) {
    let error = Rounding::new(RoundingPer::Entry, step, RoundingMode::Up).err();

    assert!(
        error.is_some_and(|error| error.is_code("pace::core::rounding-step")),
        "a step of {step} rounds nothing"
    );
}

#[rstest]
#[case("area", RoleKind::Area)]
#[case("area-project", RoleKind::AreaProject)]
#[case("meta", RoleKind::Meta)]
fn role_kind_is_written_as_word(#[case] word: &str, #[case] kind: RoleKind) -> TestResult {
    assert_eq!(word.parse::<RoleKind>()?, kind);
    assert_eq!(kind.to_string(), word);
    Ok(())
}

#[rstest]
#[case("completed", ArchiveReason::Completed)]
#[case("abandoned", ArchiveReason::Abandoned)]
#[case("ended", ArchiveReason::Ended)]
#[case("inactive", ArchiveReason::Inactive)]
fn archive_reason_is_written_as_word(
    #[case] word: &str,
    #[case] reason: ArchiveReason,
) -> TestResult {
    assert_eq!(word.parse::<ArchiveReason>()?, reason);
    assert_eq!(reason.to_string(), word);
    Ok(())
}

#[rstest]
#[case("entry", RoundingPer::Entry)]
#[case("day", RoundingPer::Day)]
fn rounding_per_is_written_as_word(#[case] word: &str, #[case] per: RoundingPer) -> TestResult {
    assert_eq!(word.parse::<RoundingPer>()?, per);
    Ok(())
}

#[rstest]
#[case("up", RoundingMode::Up)]
#[case("down", RoundingMode::Down)]
#[case("nearest", RoundingMode::Nearest)]
fn rounding_mode_is_written_as_word(#[case] word: &str, #[case] mode: RoundingMode) -> TestResult {
    assert_eq!(word.parse::<RoundingMode>()?, mode);
    Ok(())
}

#[test]
fn unknown_role_kind_names_choices() {
    let error = "project".parse::<RoleKind>().err();

    assert_eq!(
        error.as_ref().map(ToString::to_string).as_deref(),
        Some("`project` is not a role kind")
    );
    assert_eq!(
        error.as_ref().and_then(|error| error.hint()),
        Some("a role kind is one of `area`, `area-project`, `meta`")
    );
}
