//! Every event kind reads back as written, and each invalid event file is
//! refused with its reason.

use pace_core::{
    Billing, Change, DeterminedBy, EntryChanges, Event, EventKind, Hlc, Money, ProjectRef, RoleId,
};

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn id(prefix: u16, number: u32) -> String {
    format!("0193a8d0-{prefix:04x}-7000-8000-{number:012x}")
}

/// One event of each kind, with every field it can have, in three time
/// zones. A new kind belongs here.
fn one_of_each() -> Result<Vec<EventKind>, Box<dyn std::error::Error>> {
    let entry: EntryId = id(0xa, 1).parse()?;
    let tags = BTreeSet::from(["dev".parse()?]);
    let project = Some(ProjectRef::Key("acme-webshop".parse()?));
    let role: RoleId = id(0xe, 1).parse()?;
    Ok(vec![
        EventKind::EntryBegun {
            entry,
            project: project.clone(),
            note: Some("Write offer".to_owned()),
            tags: tags.clone(),
            role: Some(role),
            determined_by: Some(DeterminedBy::Others),
            billable: Some(true),
            allow_overlap: true,
        },
        EventKind::EntryHeld {
            entry,
            source: HoldSource::Outer,
            reason: Some("phone call".to_owned()),
        },
        EventKind::EntryPaused {
            entry,
            kind: PauseKind::Lunch,
            reason: Some("hungry".to_owned()),
        },
        EventKind::EntryResumed { entry },
        EventKind::EntryEnded { entry },
        EventKind::CatchNoted {
            entry,
            text: "call the tax office".to_owned(),
        },
        EventKind::EntryLogged {
            entry,
            end: "2026-11-26T15:30[Asia/Kolkata]".parse()?,
            project,
            note: Some("Review".to_owned()),
            tags: tags.clone(),
            role: Some(role),
            determined_by: Some(DeterminedBy::Me),
            billable: Some(false),
            allow_overlap: true,
        },
        EventKind::EntryAdjusted {
            entry,
            changes: EntryChanges {
                start: Some("2026-11-26T02:45[America/New_York]".parse()?),
                end: Some("2026-11-26T15:45[Asia/Kolkata]".parse()?),
                note: Change::Clear,
                project: Change::Set(ProjectRef::Id(id(0xc, 1).parse()?)),
                tags: Some(tags),
                role: Change::Set(role),
                determined_by: Change::Clear,
                billable: Change::Set(false),
                allow_overlap: true,
            },
        },
        EventKind::EntryAdjusted {
            entry,
            changes: EntryChanges {
                note: Change::Set("Offer, second draft".to_owned()),
                project: Change::Clear,
                role: Change::Clear,
                determined_by: Change::Set(DeterminedBy::Others),
                billable: Change::Clear,
                ..EntryChanges::default()
            },
        },
        EventKind::BillingMarked {
            entry,
            billing: Billing {
                invoice: "2026-0042".parse()?,
                net: jiff::SignedDuration::from_mins(50),
                billed: jiff::SignedDuration::from_hours(1),
                rate: Money::parse_decimal("95.50", "EUR".parse()?)?,
                amount: Money::parse_decimal("95.50", "EUR".parse()?)?,
            },
        },
        EventKind::EntryForgotten { entry },
        EventKind::BatchUndone {
            batch: id(0xb, 9).parse()?,
        },
    ])
}

/// Each event of `kinds`, recorded at 09:00 in Berlin about 03:30 in New
/// York.
fn recorded_events(
    kinds: Vec<EventKind>,
) -> Result<Vec<RecordedEvent>, Box<dyn std::error::Error>> {
    let device: DeviceId = id(0xd, 1).parse()?;
    let at: ZonedDateTime = "2026-11-26T09:00[Europe/Berlin]".parse()?;
    let about: ZonedDateTime = "2026-11-26T03:30[America/New_York]".parse()?;
    kinds
        .into_iter()
        .zip(1..)
        .map(|(kind, number)| {
            Ok(RecordedEvent {
                device,
                batch: id(0xb, number).parse::<BatchId>()?,
                hlc: Hlc::new(at.timestamp(), number),
                event: Event {
                    id: id(0xe, number).parse()?,
                    at: at.clone(),
                    about: about.clone(),
                    kind,
                },
            })
        })
        .collect()
}

#[test]
fn every_event_kind_reads_back_as_written() -> TestResult {
    let written = recorded_events(one_of_each()?)?;
    let text = format!("{}{}", file_header(), serialize_events(&written)?);

    let read = parse_events(Path::new("2026-11.toml"), &text, written[0].device)?;

    assert_eq!(read, written);
    // `ZonedDateTime` equality compares the instant only; the debug text
    // holds the time zone of each timestamp too.
    assert_eq!(format!("{read:#?}"), format!("{written:#?}"));
    Ok(())
}

#[test]
fn every_event_type_is_named_as_in_the_log() -> TestResult {
    for kind in one_of_each()? {
        let event_type = EventType::of(&kind);
        assert_eq!(EventType::parse(event_type.name()), Some(event_type));
    }
    assert_eq!(EventType::parse("unknown"), None);
    Ok(())
}

/// An event file of `version` with one event: its envelope, then `fields`.
fn event_file_of_version(version: &str, fields: &str) -> String {
    format!(
        "version = {version}\n\n[[event]]\nid = \"{id}\"\nbatch = \"{batch}\"\n\
         at = \"2026-11-26T09:00:00+01:00[Europe/Berlin]\"\n{fields}\n",
        id = id(0xe, 1),
        batch = id(0xb, 1),
    )
}

/// An event file with one event: its envelope, then `fields`.
fn event_file_with(fields: &str) -> String {
    event_file_of_version("1", fields)
}

/// The fields of a valid `entry.ended` event besides its envelope.
fn ended() -> String {
    format!(
        "type = \"entry.ended\"\nv = 1\nhlc = \"2026-11-26T08:00:00.000Z/0\"\nentry = \"{}\"",
        id(0xa, 1)
    )
}

/// The error of reading the event file `text`.
fn refusal(text: &str) -> Result<Box<PaceError>, Box<dyn std::error::Error>> {
    let device: DeviceId = id(0xd, 1).parse()?;
    match parse_events(Path::new("2026-11.toml"), text, device) {
        Ok(events) => Err(format!("the file must be refused, but reads as {events:?}").into()),
        Err(error) => Ok(error),
    }
}

#[test]
fn event_file_reads_its_one_event() -> TestResult {
    let device: DeviceId = id(0xd, 1).parse()?;

    let events = parse_events(
        Path::new("2026-11.toml"),
        &event_file_with(&ended()),
        device,
    )?;

    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].event.kind,
        EventKind::EntryEnded {
            entry: id(0xa, 1).parse()?
        }
    );
    Ok(())
}

#[test]
fn event_file_of_newer_version_asks_for_update() -> TestResult {
    let error = refusal(&event_file_of_version("2", &ended()))?;

    assert_eq!(
        error.to_string(),
        "this pace reads version 1 of event files, not 2"
    );
    assert_eq!(error.hint(), Some(NEWER));
    assert!(error.is_code("pace::store::version"));
    Ok(())
}

#[test]
fn event_file_of_older_version_is_for_reading() -> TestResult {
    let error = refusal(&event_file_of_version("0", &ended()))?;

    assert_eq!(
        error.to_string(),
        "this pace reads version 1 of event files, not 0"
    );
    assert_eq!(error.hint(), Some(READ_ONLY));
    Ok(())
}

#[test]
fn event_file_with_unknown_top_level_key_names_it() -> TestResult {
    let text = format!("events = []\n{}", event_file_with(&ended()));

    let error = refusal(&text)?;

    assert_eq!(
        error.to_string(),
        "`events` is not a key pace knows in an event"
    );
    assert_eq!(
        error.hint(),
        Some(format!("did you mean `event`? {READ_ONLY}").as_str())
    );
    assert!(error.is_code("pace::store::unknown-key"));
    Ok(())
}

#[test]
fn event_file_key_unlike_any_known_key_gets_no_suggestion() -> TestResult {
    let error = refusal(&event_file_with(&format!(
        "{}\nwhereabouts = \"x\"",
        ended()
    )))?;

    assert_eq!(
        error.to_string(),
        "`whereabouts` is not a key pace knows in an event"
    );
    assert_eq!(error.hint(), Some(READ_ONLY));
    Ok(())
}

#[test]
fn event_file_whose_event_is_no_table_cannot_be_read() -> TestResult {
    let error = refusal("version = 1\nevent = [1]\n")?;

    assert_eq!(
        error.to_string(),
        "pace cannot read the event file 2026-11.toml"
    );
    assert_eq!(error.hint(), Some(REPAIR));
    assert!(error.is_code("pace::store::invalid-file"));
    Ok(())
}

#[test]
fn event_with_broken_hlc_names_it() -> TestResult {
    let fields = ended().replace("2026-11-26T08:00:00.000Z/0", "08:00/zero");

    let error = refusal(&event_file_with(&fields))?;

    assert_eq!(
        error.to_string(),
        "`08:00/zero` is no HLC like `2026-11-26T08:02:11.204Z/0`"
    );
    assert_eq!(error.hint(), Some(READ_ONLY));
    assert!(error.is_code("pace::store::invalid-event"));
    Ok(())
}

#[test]
fn event_of_newer_version_asks_for_update() -> TestResult {
    let error = refusal(&event_file_with(&ended().replace("v = 1", "v = 2")))?;

    assert_eq!(
        error.to_string(),
        "pace reads version 1 of `entry.ended`, not 2"
    );
    assert_eq!(error.hint(), Some(NEWER));
    Ok(())
}

#[test]
fn event_of_older_version_is_for_reading() -> TestResult {
    let error = refusal(&event_file_with(&ended().replace("v = 1", "v = 0")))?;

    assert_eq!(
        error.to_string(),
        "pace reads version 1 of `entry.ended`, not 0"
    );
    assert_eq!(error.hint(), Some(READ_ONLY));
    Ok(())
}

#[test]
fn event_with_field_of_another_type_names_it() -> TestResult {
    let error = refusal(&event_file_with(&format!("{}\nnote = \"x\"", ended())))?;

    assert_eq!(
        error.to_string(),
        "an event of type `entry.ended` has no `note`"
    );
    assert_eq!(error.hint(), Some(READ_ONLY));
    Ok(())
}

#[test]
fn event_with_project_by_id_and_by_key_needs_one() -> TestResult {
    let fields = ended().replace("entry.ended", "entry.begun")
        + &format!(
            "\nproject = \"{}\"\nproject-key = \"acme-webshop\"",
            id(0xc, 1)
        );

    let error = refusal(&event_file_with(&fields))?;

    assert_eq!(
        error.to_string(),
        "an event of type `entry.begun` has `project` and `project-key`; it needs one"
    );
    Ok(())
}

/// The fields of an `entry.adjusted` event with `changes`.
fn adjusted_with(changes: &str) -> String {
    format!(
        "{}\n{changes}",
        ended().replace("entry.ended", "entry.adjusted")
    )
}

#[test]
fn adjusted_event_clears_only_note_project_role_determined_by_and_billable() -> TestResult {
    let error = refusal(&event_file_with(&adjusted_with("cleared = [\"tags\"]")))?;

    assert_eq!(
        error.to_string(),
        "`cleared` names `tags`; only `note`, `project`, `role`, `determined-by` and `billable` \
         can be cleared"
    );
    assert_eq!(error.context_value("field"), Some("tags"));
    Ok(())
}

#[test]
fn adjusted_event_cannot_set_and_clear_note_at_once() -> TestResult {
    let error = refusal(&event_file_with(&adjusted_with(
        "note = \"x\"\ncleared = [\"note\"]",
    )))?;

    assert_eq!(error.to_string(), "`note` is set and cleared at once");
    assert_eq!(error.context_value("field"), Some("note"));
    Ok(())
}

/// The fields of a `billing.marked` event: 50 minutes billed as an hour at
/// 95.50 EUR, with `amount` as the amount.
fn marked_with(currency: &str, amount: &str) -> String {
    format!(
        "{}\ninvoice = \"2026-0042\"\nnet-seconds = 3000\nbilled-seconds = 3600\n\
         rate = \"95.50\"\ncurrency = \"{currency}\"\namount = \"{amount}\"",
        ended().replace("entry.ended", "billing.marked")
    )
}

#[test]
fn billing_marked_reads_its_frozen_values() -> TestResult {
    let device: DeviceId = id(0xd, 1).parse()?;

    let events = parse_events(
        Path::new("2026-11.toml"),
        &event_file_with(&marked_with("EUR", "95.50")),
        device,
    )?;

    let eur = "EUR".parse()?;
    assert_eq!(
        events[0].event.kind,
        EventKind::BillingMarked {
            entry: id(0xa, 1).parse()?,
            billing: Billing {
                invoice: "2026-0042".parse()?,
                net: jiff::SignedDuration::from_mins(50),
                billed: jiff::SignedDuration::from_hours(1),
                rate: Money::from_minor(9550, eur),
                amount: Money::from_minor(9550, eur),
            },
        }
    );
    Ok(())
}

#[test]
fn billing_marked_needs_its_amount() -> TestResult {
    let text = event_file_with(&marked_with("EUR", "95.50")).replace("amount = \"95.50\"\n", "");

    let error = refusal(&text)?;

    assert_eq!(
        error.to_string(),
        "an event of type `billing.marked` needs `amount`"
    );
    Ok(())
}

#[test]
fn billing_marked_with_unknown_currency_names_it() -> TestResult {
    let error = refusal(&event_file_with(&marked_with("EUX", "95.50")))?;

    assert_eq!(
        error.to_string(),
        "`EUX` in `currency` is no currency pace knows"
    );
    assert_eq!(error.context_value("currency"), Some("EUX"));
    Ok(())
}

#[test]
fn billing_marked_with_amount_finer_than_currency_names_it() -> TestResult {
    let error = refusal(&event_file_with(&marked_with("EUR", "95.505")))?;

    assert_eq!(
        error.to_string(),
        "`95.505` in `amount` is no amount in EUR"
    );
    assert_eq!(error.context_value("value"), Some("95.505"));
    Ok(())
}

#[test]
fn billing_marked_with_blank_invoice_names_it() -> TestResult {
    let text = event_file_with(&marked_with("EUR", "95.50"))
        .replace("invoice = \"2026-0042\"", "invoice = \" \"");

    let error = refusal(&text)?;

    assert_eq!(error.to_string(), "\" \" is no invoice reference");
    Ok(())
}

/// An event file with a valid event between two invalid ones: one of an
/// unknown type, one with an unknown key.
fn file_with_two_invalid_events() -> String {
    let unknown_type = ended().replace("entry.ended", "entry.bogun");
    let unknown_key = format!("{}\nwhereabouts = \"x\"", ended());
    let event = |fields: &str| {
        format!(
            "[[event]]\nid = \"{id}\"\nbatch = \"{batch}\"\n\
             at = \"2026-11-26T09:00:00+01:00[Europe/Berlin]\"\n{fields}\n\n",
            id = id(0xe, 1),
            batch = id(0xb, 1),
        )
    };
    format!(
        "version = 1\n\n{}{}{}",
        event(&unknown_type),
        event(&ended()),
        event(&unknown_key)
    )
}

#[test]
fn collecting_reader_names_every_invalid_event_with_its_line() -> TestResult {
    let device: DeviceId = id(0xd, 1).parse()?;

    let read = parse_events_collecting(
        Path::new("2026-11.toml"),
        &file_with_two_invalid_events(),
        device,
    );

    assert_eq!(read.events.len(), 1);
    let found: Vec<(String, Option<usize>)> = read
        .mistakes
        .iter()
        .map(|mistake| (mistake.to_string(), mistake.snippet_line()))
        .collect();
    assert_eq!(
        found,
        [
            (
                "pace doesn't know the event type `entry.bogun`".to_owned(),
                Some(3)
            ),
            (
                "`whereabouts` is not a key pace knows in an event".to_owned(),
                Some(29)
            ),
        ]
    );
    Ok(())
}

#[test]
fn collecting_reader_of_broken_toml_names_one_mistake() -> TestResult {
    let device: DeviceId = id(0xd, 1).parse()?;

    let read = parse_events_collecting(Path::new("2026-11.toml"), "version = \n", device);

    assert_eq!(read.events, []);
    assert_eq!(read.mistakes.len(), 1);
    assert!(read.mistakes[0].is_code("pace::store::syntax"));
    Ok(())
}

#[test]
fn collecting_reader_of_valid_file_finds_no_mistake() -> TestResult {
    let device: DeviceId = id(0xd, 1).parse()?;

    let read = parse_events_collecting(
        Path::new("2026-11.toml"),
        &event_file_with(&ended()),
        device,
    );

    assert_eq!(read.events.len(), 1);
    assert_eq!(read.mistakes.len(), 0);
    Ok(())
}

#[test]
fn collecting_reader_names_value_of_wrong_type_and_reads_the_rest() -> TestResult {
    let device: DeviceId = id(0xd, 1).parse()?;
    let wrong = event_file_with(&ended().replace("v = 1", "v = \"one\""));
    let text = format!(
        "{wrong}\n{}",
        event_file_with(&ended()).replace("version = 1\n", "")
    );

    let read = parse_events_collecting(Path::new("2026-11.toml"), &text, device);

    assert_eq!(read.events.len(), 1);
    assert_eq!(read.mistakes.len(), 1);
    assert!(
        read.mistakes[0]
            .to_string()
            .starts_with("pace cannot read this event: "),
        "{}",
        read.mistakes[0]
    );
    assert_eq!(read.mistakes[0].snippet_line(), Some(3));
    Ok(())
}
