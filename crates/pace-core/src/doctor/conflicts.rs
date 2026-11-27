//! The check of `pace doctor` for fields that two devices changed: per
//! field, the later change by HLC counts, and doctor names every value.
//!
//! The HLC orders the changes but doesn't record what a device had seen
//! when it changed a field. So a change made after a sync of the other
//! device's change is listed too, and the finding says that it can be no
//! conflict.

use std::collections::{BTreeMap, BTreeSet};

use pace_time::format_moment;

use crate::{
    BatchId, Change, DeterminedBy, DeviceId, EntryChanges, EntryId, EventKind, ProjectRef,
    RecordedEvent, RoleId,
    doctor::{DataArea, DataFinding, DoctorInput},
};

/// The fields of an entry that a change sets, in the order the findings
/// name them.
const FIELDS: [&str; 8] = [
    "start",
    "end",
    "note",
    "project",
    "tags",
    "role",
    "determined-by",
    "billable",
];

/// One device's change of one field, with the value as text.
struct FieldChange {
    device: DeviceId,
    value: String,
}

/// A warning for each field of an entry that more than one device
/// changed, with the last value of each device and the one that counts.
///
/// A change is an `entry.adjusted` that sets the field, or an
/// `entry.ended`, which sets the end. Changes in undone batches and of
/// entries the state doesn't hold are left out.
pub(crate) fn changed_on_two_devices(input: &DoctorInput<'_>) -> Vec<DataFinding> {
    let undone: BTreeSet<BatchId> = input
        .events
        .iter()
        .filter_map(|recorded| match recorded.event.kind {
            EventKind::BatchUndone { batch } => Some(batch),
            _ => None,
        })
        .collect();
    let mut ordered: Vec<&RecordedEvent> = input
        .events
        .iter()
        .filter(|recorded| !undone.contains(&recorded.batch))
        .collect();
    ordered.sort_by_key(|recorded| recorded.merge_key());

    let mut changes: BTreeMap<(EntryId, usize), Vec<FieldChange>> = BTreeMap::new();
    for recorded in ordered {
        for (entry, field, value) in field_changes(input, recorded) {
            if input.state.entry(entry).is_some() {
                changes
                    .entry((entry, field))
                    .or_default()
                    .push(FieldChange {
                        device: recorded.device,
                        value,
                    });
            }
        }
    }
    changes
        .into_iter()
        .filter_map(|((entry, field), changes)| conflict(input, entry, FIELDS[field], &changes))
        .collect()
}

/// The finding for the `changes` of `field` of `entry`, in merged order,
/// if more than one device made them.
fn conflict(
    input: &DoctorInput<'_>,
    entry: EntryId,
    field: &str,
    changes: &[FieldChange],
) -> Option<DataFinding> {
    // The last change of each device, in the order of those last changes.
    let mut last: Vec<&FieldChange> = Vec::new();
    for change in changes {
        last.retain(|kept| kept.device != change.device);
        last.push(change);
    }
    let counts = last.last().filter(|_| last.len() > 1)?;
    let short = input.state.short_id(entry);
    let values: Vec<String> = last
        .iter()
        .map(|change| format!("{} on device {}", change.value, change.device))
        .collect();
    Some(
        DataFinding::warning(
            DataArea::Entries,
            format!(
                "the {field} of {short} changed on {} devices: {}; {} counts, the later change",
                last.len(),
                values.join(", "),
                counts.value
            ),
            format!(
                "pace doesn't record what a device had seen, so this can be no conflict; if \
                 another value is right, set it with `pace edit {short}`"
            ),
        )
        .about(short),
    )
}

/// The fields `recorded` sets, each as the entry, the field's place in
/// [`FIELDS`] and its new value as text.
fn field_changes(
    input: &DoctorInput<'_>,
    recorded: &RecordedEvent,
) -> Vec<(EntryId, usize, String)> {
    match &recorded.event.kind {
        EventKind::EntryEnded { entry } => {
            vec![(*entry, 1, format_moment(&recorded.event.at, input.now))]
        }
        EventKind::EntryAdjusted { entry, changes } => adjusted_fields(input, changes)
            .into_iter()
            .map(|(field, value)| (*entry, field, value))
            .collect(),
        _ => Vec::new(),
    }
}

/// The fields `changes` sets, by their place in [`FIELDS`], with their
/// values as text.
fn adjusted_fields(input: &DoctorInput<'_>, changes: &EntryChanges) -> Vec<(usize, String)> {
    let moment = |moment: &jiff::ZonedDateTime| format_moment(moment, input.now);
    let tags = changes.tags.as_ref().map(|tags| {
        if tags.is_empty() {
            "no tags".to_owned()
        } else {
            let tags: Vec<String> = tags.iter().map(|tag| format!("#{tag}")).collect();
            tags.join(" ")
        }
    });
    [
        changes.start.as_ref().map(moment),
        changes.end.as_ref().map(moment),
        change_text(&changes.note, |note| format!("\"{note}\""), "no note"),
        change_text(
            &changes.project,
            |project| project_name(input, project),
            "no project",
        ),
        tags,
        change_text(
            &changes.role,
            |role| role_name(input, *role),
            "the project's role",
        ),
        change_text(
            &changes.determined_by,
            |by| determined_text(*by).to_owned(),
            "unmarked",
        ),
        change_text(
            &changes.billable,
            |billable| {
                if *billable {
                    "billable"
                } else {
                    "not billable"
                }
                .to_owned()
            },
            "the project's flag",
        ),
    ]
    .into_iter()
    .enumerate()
    .filter_map(|(field, value)| value.map(|value| (field, value)))
    .collect()
}

/// The text of `change`: its value through `text`, `cleared` for a
/// change that empties the field, `None` for one that keeps it.
fn change_text<T>(
    change: &Change<T>,
    text: impl FnOnce(&T) -> String,
    cleared: &str,
) -> Option<String> {
    match change {
        Change::Keep => None,
        Change::Set(value) => Some(text(value)),
        Change::Clear => Some(cleared.to_owned()),
    }
}

/// The key of the project an entry refers to: its record's, else the
/// key the entry names, else the ID.
fn project_name(input: &DoctorInput<'_>, project: &ProjectRef) -> String {
    match (input.catalog.resolve(project), project) {
        (Some(record), _) => format!("`{}`", record.key),
        (None, ProjectRef::Key(key)) => format!("`{key}`"),
        (None, ProjectRef::Id(id)) => id.to_string(),
    }
}

/// The key of the role `id`, else the ID.
fn role_name(input: &DoctorInput<'_>, id: RoleId) -> String {
    input
        .catalog
        .role_by_id(id)
        .map_or_else(|| id.to_string(), |role| format!("`{}`", role.key))
}

/// Who determined the time, as a value of a finding.
const fn determined_text(by: DeterminedBy) -> &'static str {
    match by {
        DeterminedBy::Me => "self-determined",
        DeterminedBy::Others => "other-determined",
    }
}
