//! Commands are data: the CLI, a later daemon and plugins exchange the same
//! JSON. These tests pin that wire format.

use std::collections::BTreeSet;

use pace_core::{
    Change, Command, DeterminedBy, HoldSource, PauseKind, ProjectRef,
    command::{Adjust, Begin, Catch, End, Forget, Hold, Log, Pause, Resume},
};
use rstest::rstest;

use crate::TestResult;

fn round_trip(command: &Command) -> Result<String, Box<dyn std::error::Error>> {
    let json = serde_json::to_string(command)?;
    assert_eq!(&serde_json::from_str::<Command>(&json)?, command);
    Ok(json)
}

#[test]
fn begin_carries_what_was_typed() -> TestResult {
    let command = Command::Begin(Begin {
        at: Some("2026-11-26T09:00:00+01:00[Europe/Berlin]".parse()?),
        note: Some("Write offer".parse()?),
        project: Some(ProjectRef::Key("acme-webshop".parse()?)),
        tags: BTreeSet::from(["dev".parse()?, "offer".parse()?]),
        role: Some("0193a8d0-000e-7000-8000-000000000001".parse()?),
        determined_by: Some(DeterminedBy::Others),
        billable: Some(false),
        allow_overlap: true,
    });

    assert_eq!(
        round_trip(&command)?,
        r#"{"command":"begin","at":"2026-11-26T09:00:00+01:00[Europe/Berlin]","note":"Write offer","project":{"key":"acme-webshop"},"tags":["dev","offer"],"role":"0193a8d0-000e-7000-8000-000000000001","determined-by":"other","billable":false,"allow-overlap":true}"#
    );
    Ok(())
}

#[rstest]
#[case::begin(Command::Begin(Begin::default()), r#"{"command":"begin"}"#)]
#[case::hold(Command::Hold(Hold::default()), r#"{"command":"hold"}"#)]
#[case::pause(Command::Pause(Pause::default()), r#"{"command":"pause"}"#)]
#[case::resume(Command::Resume(Resume::default()), r#"{"command":"resume"}"#)]
#[case::end(Command::End(End::default()), r#"{"command":"end"}"#)]
#[case::adjust(Command::Adjust(Adjust::default()), r#"{"command":"adjust"}"#)]
#[case::undo(Command::Undo, r#"{"command":"undo"}"#)]
fn defaults_are_left_out(#[case] command: Command, #[case] json: &str) -> TestResult {
    assert_eq!(round_trip(&command)?, json);
    Ok(())
}

#[test]
fn hold_says_where_the_interruption_came_from() -> TestResult {
    let command = Command::Hold(Hold {
        at: None,
        source: Some(HoldSource::Outer),
        reason: Some("phone call".parse()?),
    });

    assert_eq!(
        round_trip(&command)?,
        r#"{"command":"hold","source":"outer","reason":"phone call"}"#
    );
    Ok(())
}

#[test]
fn pause_names_its_kind() -> TestResult {
    let command = Command::Pause(Pause {
        at: None,
        kind: PauseKind::Lunch,
        reason: None,
    });

    assert_eq!(
        round_trip(&command)?,
        r#"{"command":"pause","kind":"lunch"}"#
    );
    Ok(())
}

#[test]
fn catch_carries_the_thought() -> TestResult {
    let command = Command::Catch(Catch {
        text: "call the tax office".parse()?,
    });

    assert_eq!(
        round_trip(&command)?,
        r#"{"command":"catch","text":"call the tax office"}"#
    );
    Ok(())
}

#[test]
fn project_is_named_by_id_or_by_key() -> TestResult {
    let command = Command::Begin(Begin {
        project: Some(ProjectRef::Id(
            "0193a7c4-6c1e-7f3a-9b2d-4e5f60718293".parse()?,
        )),
        ..Begin::default()
    });

    assert_eq!(
        round_trip(&command)?,
        r#"{"command":"begin","project":{"id":"0193a7c4-6c1e-7f3a-9b2d-4e5f60718293"}}"#
    );
    Ok(())
}

#[test]
fn log_carries_start_and_end() -> TestResult {
    let command = Command::Log(Log::new(
        "2026-11-25T14:00:00+01:00[Europe/Berlin]".parse()?,
        "2026-11-25T15:30:00+01:00[Europe/Berlin]".parse()?,
    ));

    assert_eq!(
        round_trip(&command)?,
        r#"{"command":"log","start":"2026-11-25T14:00:00+01:00[Europe/Berlin]","end":"2026-11-25T15:30:00+01:00[Europe/Berlin]"}"#
    );
    Ok(())
}

#[test]
fn adjust_sets_with_value_and_clears_with_null() -> TestResult {
    let command = Command::Adjust(Adjust {
        note: Change::Set("Write offer".parse()?),
        project: Change::Clear,
        tag: BTreeSet::from(["dev".parse()?]),
        ..Adjust::default()
    });

    assert_eq!(
        round_trip(&command)?,
        r#"{"command":"adjust","note":"Write offer","project":null,"tag":["dev"]}"#
    );
    Ok(())
}

#[test]
fn adjust_changes_role_determined_by_and_billable_like_note() -> TestResult {
    let command = Command::Adjust(Adjust {
        role: Change::Clear,
        determined_by: Change::Set(DeterminedBy::Me),
        billable: Change::Set(true),
        allow_overlap: true,
        ..Adjust::default()
    });

    assert_eq!(
        round_trip(&command)?,
        r#"{"command":"adjust","role":null,"determined-by":"self","billable":true,"allow-overlap":true}"#
    );
    Ok(())
}

#[test]
fn forget_names_the_entry() -> TestResult {
    let command = Command::Forget(Forget {
        entry: "0193a8d0-6c1e-7f3a-9b2d-4e5f60718293".parse()?,
    });

    assert_eq!(
        round_trip(&command)?,
        r#"{"command":"forget","entry":"0193a8d0-6c1e-7f3a-9b2d-4e5f60718293"}"#
    );
    Ok(())
}

#[rstest]
#[case::end(r#"["end"]"#, Command::End(End::default()))]
#[case::undo(r#"["undo"]"#, Command::Undo)]
fn command_without_fields_reads_from_sequence(
    #[case] json: &str,
    #[case] command: Command,
) -> TestResult {
    assert_eq!(serde_json::from_str::<Command>(json)?, command);
    Ok(())
}

#[rstest]
#[case::unknown_field_of_end(r#"{"command":"end","force":true}"#, "unknown field `force`")]
#[case::unknown_field_of_undo(r#"{"command":"undo","force":true}"#, "unknown field `force`")]
#[case::undo_sequence_with_element(r#"["undo",true]"#, "invalid length 1, expected no fields")]
#[case::unknown_command(r#"{"command":"stop"}"#, "unknown variant `stop`")]
#[case::project_without_key_or_id(
    r#"{"command":"begin","project":"acme-webshop"}"#,
    "expected `id` or `key`"
)]
#[case::invalid_key(
    r#"{"command":"begin","project":{"key":"Acme Webshop"}}"#,
    "`Acme Webshop` is not a project key"
)]
#[case::catch_without_text(r#"{"command":"catch"}"#, "missing field `text`")]
#[case::catch_with_empty_text(r#"{"command":"catch","text":""}"#, r#"the text "" has no words"#)]
#[case::hold_with_blank_reason(
    r#"{"command":"hold","reason":"  "}"#,
    r#"the text "  " has no words"#
)]
#[case::pause_with_empty_reason(
    r#"{"command":"pause","reason":""}"#,
    r#"the text "" has no words"#
)]
#[case::begin_with_empty_note(r#"{"command":"begin","note":""}"#, r#"the text "" has no words"#)]
#[case::log_with_blank_note(r#"{"command":"log","start":"2026-11-25T14:00:00+01:00[Europe/Berlin]","end":"2026-11-25T15:00:00+01:00[Europe/Berlin]","note":" "}"#, r#"the text " " has no words"#)]
#[case::adjust_with_empty_note(r#"{"command":"adjust","note":""}"#, r#"the text "" has no words"#)]
fn malformed_commands_are_rejected_with_reason(#[case] json: &str, #[case] reason: &str) {
    let error = serde_json::from_str::<Command>(json).err();

    let message = error.map_or_else(String::new, |error| error.to_string());
    assert!(message.contains(reason), "{message}");
}
