//! The catalog: roles, clients and project records, their keys and rates.

use pace_core::{ClientKey, RoleId, RoleKey};
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
fn client_key_with_space_is_refused_with_an_example() {
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
