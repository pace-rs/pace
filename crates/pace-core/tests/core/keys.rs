//! Keys are what people type for projects; tags label entries.

use pace_core::{ProjectKey, Tag};
use rstest::rstest;

use crate::TestResult;

#[rstest]
#[case("acme-webshop")]
#[case("garden")]
#[case("2026-taxes")]
fn project_key_is_lowercase_words_with_dashes(#[case] key: &str) -> TestResult {
    assert_eq!(key.parse::<ProjectKey>()?.as_str(), key);
    Ok(())
}

#[rstest]
#[case::empty("")]
#[case::uppercase("Acme")]
#[case::space("acme webshop")]
#[case::underscore("acme_webshop")]
#[case::leading_dash("-acme")]
#[case::trailing_dash("acme-")]
#[case::double_dash("acme--webshop")]
#[case::umlaut("büro")]
fn other_project_keys_are_rejected_with_an_example(#[case] key: &str) {
    let hint = key
        .parse::<ProjectKey>()
        .err()
        .and_then(|error| error.hint().map(ToOwned::to_owned));

    assert_eq!(
        hint.as_deref(),
        Some("a key has lowercase letters, digits and single dashes, like `acme-webshop`")
    );
}

#[rstest]
#[case("dev")]
#[case("deep-work")]
#[case("Café")]
fn tag_is_one_word(#[case] tag: &str) -> TestResult {
    assert_eq!(tag.parse::<Tag>()?.as_str(), tag);
    Ok(())
}

#[rstest]
#[case::empty("")]
#[case::space("deep work")]
#[case::comma("dev,ops")]
fn tag_with_spaces_or_commas_is_rejected(#[case] tag: &str) {
    assert!(tag.parse::<Tag>().is_err());
}
