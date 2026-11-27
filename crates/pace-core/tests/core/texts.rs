//! A text says something: the note of an entry, the reason of a hold or a
//! pause, a caught thought.

use pace_core::Text;
use rstest::rstest;

use crate::TestResult;

#[rstest]
#[case("call the tax office")]
#[case(" Write offer ")]
fn text_is_kept_as_written(#[case] text: &str) -> TestResult {
    assert_eq!(text.parse::<Text>()?.as_str(), text);
    Ok(())
}

#[rstest]
#[case::empty("")]
#[case::spaces("   ")]
#[case::tab_and_newline("\t\n")]
fn text_without_words_is_refused(#[case] text: &str) {
    let error = text.parse::<Text>().err();

    assert_eq!(
        error.as_ref().map(ToString::to_string),
        Some(format!("the text \"{text}\" has no words"))
    );
}
