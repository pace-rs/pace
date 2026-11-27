//! The entry checks of `pace doctor`: several open entries, overlaps that
//! nothing allows, fields changed on two devices, and entries whose
//! project key has no record.

use std::collections::BTreeMap;

use crate::{
    Entry, ProjectKey, ProjectRef,
    bill::shared_time,
    doctor::{
        DataArea, DataFinding, DoctorInput, FindingSeverity, conflicts::changed_on_two_devices,
    },
};

/// Every finding about the entries: several open entries, the overlaps,
/// the fields changed on two devices, then the project keys without a
/// record.
#[must_use]
pub fn entry_findings(input: &DoctorInput<'_>) -> Vec<DataFinding> {
    let mut findings: Vec<DataFinding> = several_open(input).into_iter().collect();
    findings.extend(overlaps(input));
    findings.extend(changed_on_two_devices(input));
    findings.extend(keys_without_record(input));
    findings
}

/// The error for more than one open entry: commands that work on the
/// open entry refuse to guess. Two devices that each began an entry
/// before they synced leave two.
fn several_open(input: &DoctorInput<'_>) -> Option<DataFinding> {
    let state = input.state;
    let open: Vec<&Entry> = state.open().collect();
    if open.len() < 2 {
        return None;
    }
    let labels: Vec<String> = open
        .iter()
        .map(|entry| state.entry_label(entry, input.now))
        .collect();
    Some(DataFinding {
        severity: FindingSeverity::Error,
        ..DataFinding::warning(
            DataArea::Entries,
            format!(
                "{} entries are open at once: {}",
                open.len(),
                labels.join("; ")
            ),
            "end all but one with `pace end <id>`".to_owned(),
        )
    })
}

/// A warning for each pair of entries that share time while neither of
/// them nor its project allows it; a pair of open entries is the error of
/// [`several_open`] already.
fn overlaps(input: &DoctorInput<'_>) -> Vec<DataFinding> {
    let state = input.state;
    state
        .overlaps(input.now)
        .into_iter()
        .filter(|overlap| !(overlap.first.is_open() && overlap.second.is_open()))
        .filter(|overlap| {
            !overlap.first.allows_overlap(input.catalog)
                && !overlap.second.allows_overlap(input.catalog)
        })
        .map(|overlap| {
            let first = state.short_id(overlap.first.id);
            DataFinding::warning(
                DataArea::Entries,
                format!(
                    "the entries {first} and {} overlap: they share {}",
                    state.short_id(overlap.second.id),
                    shared_time(&overlap.start, &overlap.end, input.zone)
                ),
                "move one with `pace adjust <id> --start` or `--end`, or allow the overlap with \
                 `pace adjust <id> --allow-overlap`"
                    .to_owned(),
            )
            .about(first)
        })
        .collect()
}

/// A warning for each project key that entries name but no record has,
/// with the entries by start.
fn keys_without_record(input: &DoctorInput<'_>) -> Vec<DataFinding> {
    let state = input.state;
    let mut by_key: BTreeMap<&ProjectKey, Vec<String>> = BTreeMap::new();
    for entry in state.entries() {
        if let Some(ProjectRef::Key(key)) = &entry.project
            && input.catalog.project(key).is_none()
        {
            by_key
                .entry(key)
                .or_default()
                .push(state.short_id(entry.id));
        }
    }
    by_key
        .into_iter()
        .map(|(key, entries)| {
            let count = if entries.len() == 1 {
                "1 entry names".to_owned()
            } else {
                format!("{} entries name", entries.len())
            };
            DataFinding::warning(
                DataArea::Entries,
                format!(
                    "{count} the project `{key}`, which has no record: {}",
                    entries.join(", ")
                ),
                format!(
                    "give it a record with `pace project init` in the project's directory, or \
                     with `pace project add {key}`"
                ),
            )
        })
        .collect()
}
