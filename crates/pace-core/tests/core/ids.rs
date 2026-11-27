//! All IDs are UUIDv7.

use jiff::ZonedDateTime;
use pace_core::{EntryId, Ids, ProjectId, testing::SequentialIds};
use pace_error::ErrorKind;

use crate::TestResult;

const V7: &str = "0193a8d0-6c1e-7f3a-9b2d-4e5f60718293";
const V4: &str = "9b2d4e5f-6071-4829-a3b4-c5d6e7f80912";

#[test]
fn an_id_reads_and_writes_its_uuid() -> TestResult {
    let id: EntryId = V7.parse()?;

    assert_eq!(id.to_string(), V7);
    Ok(())
}

#[test]
fn an_id_that_is_no_uuidv7_is_rejected() {
    let error = V4.parse::<ProjectId>().err();

    assert_eq!(
        error.as_ref().map(ToString::to_string).as_deref(),
        Some("`9b2d4e5f-6071-4829-a3b4-c5d6e7f80912` is not a UUIDv7")
    );
    assert_eq!(
        error.as_ref().and_then(|error| error.hint()),
        Some("pace writes its IDs itself; copy the ID pace printed")
    );
}

#[test]
fn id_source_that_gives_no_uuidv7_is_internal_error() -> TestResult {
    let error = EntryId::try_from(uuid::Uuid::parse_str(V4)?).err();

    assert_eq!(
        error.as_ref().map(ToString::to_string).as_deref(),
        Some("an ID source gave `9b2d4e5f-6071-4829-a3b4-c5d6e7f80912`, which is no UUIDv7")
    );
    assert_eq!(error.map(|error| error.kind()), Some(ErrorKind::Internal));
    Ok(())
}

#[test]
fn text_that_is_no_uuid_is_rejected() {
    assert!("acme-webshop".parse::<EntryId>().is_err());
}

#[test]
fn an_id_is_stored_as_its_text() -> TestResult {
    let id: EntryId = V7.parse()?;

    assert_eq!(serde_json::to_string(&id)?, format!("\"{V7}\""));
    assert_eq!(serde_json::from_str::<EntryId>(&format!("\"{V7}\""))?, id);
    Ok(())
}

#[test]
fn stored_id_that_is_no_uuidv7_is_rejected() {
    let error = serde_json::from_str::<EntryId>(&format!("\"{V4}\"")).err();

    assert_eq!(
        error.map(|error| error.to_string()).as_deref(),
        Some("`9b2d4e5f-6071-4829-a3b4-c5d6e7f80912` is not a UUIDv7")
    );
}

// sequential IDs, for tests

fn start_of_day() -> Result<ZonedDateTime, jiff::Error> {
    "2026-11-26T08:00[Europe/Berlin]".parse()
}

#[test]
fn sequential_ids_differ_by_seed() -> TestResult {
    let laptop = SequentialIds::starting_at(&start_of_day()?, 1);
    let desktop = SequentialIds::starting_at(&start_of_day()?, 2);

    assert_ne!(laptop.next_id(), desktop.next_id());
    Ok(())
}

#[test]
fn sequential_ids_are_uuidv7() -> TestResult {
    let ids = SequentialIds::starting_at(&start_of_day()?, 0);

    assert_eq!(ids.next_id().get_version_num(), 7);
    Ok(())
}

#[test]
fn sequential_ids_never_repeat() -> TestResult {
    let ids = SequentialIds::starting_at(&start_of_day()?, 0);

    assert_ne!(ids.next_id(), ids.next_id());
    Ok(())
}
