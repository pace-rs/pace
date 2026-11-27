//! The summary of `pace setup`: each change it writes, as one line per
//! file and field, such as `pace.global.toml: time-zone system →
//! Europe/Vienna`.

use std::path::Path;

use pace_store::field_edit::{FieldChange, FieldEdit, FieldValue};

/// One change that setup writes: its field edit, and what the field held
/// before, if it held something.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupChange {
    /// The edit that writes it.
    pub edit: FieldEdit,
    /// The field's value before, as the summary shows it; `None` for a
    /// new field.
    pub before: Option<String>,
}

impl SetupChange {
    /// A new field, which held nothing before.
    #[must_use]
    pub const fn added(edit: FieldEdit) -> Self {
        Self { edit, before: None }
    }

    /// A field that held `before`.
    #[must_use]
    pub fn replacing(edit: FieldEdit, before: impl Into<String>) -> Self {
        Self {
            edit,
            before: Some(before.into()),
        }
    }
}

/// How the summary names `file`: its path in `home`, such as
/// `projects/webshop.toml`, else its file name, such as `pace.global.toml`.
#[must_use]
pub fn file_label(file: &Path, home: &Path) -> String {
    file.strip_prefix(home)
        .ok()
        .or_else(|| file.file_name().map(Path::new))
        .unwrap_or(file)
        .display()
        .to_string()
}

/// The summary's line of `change`: the file, the field, what it held and
/// what it gets.
#[must_use]
pub fn change_line(change: &SetupChange, home: &Path) -> String {
    let file = file_label(&change.edit.file, home);
    let field = change.edit.key.join(".");
    let after = match &change.edit.change {
        FieldChange::Set(value) => shown_value(value),
        FieldChange::Remove => "removed".to_owned(),
        FieldChange::AppendTable(fields) => {
            return format!("{file}: {field} + {}", shown_fields(fields));
        }
        FieldChange::ReplaceTable(_) => "replaced".to_owned(),
    };
    change.before.as_ref().map_or_else(
        || format!("{file}: {field} → {after}"),
        |before| format!("{file}: {field} {before} → {after}"),
    )
}

/// `value` as the summary shows it: text without quotes.
fn shown_value(value: &FieldValue) -> String {
    match value {
        FieldValue::String(text) | FieldValue::Decimal(text) => text.clone(),
        FieldValue::Integer(number) => number.to_string(),
        FieldValue::Boolean(yes) => yes.to_string(),
        FieldValue::Date(day) => day.to_string(),
        FieldValue::Strings(texts) => texts.join(", "),
        FieldValue::InlineTable(fields) => shown_fields(fields),
    }
}

/// `fields` as the summary shows them: `hourly 95.50, from 2026-11-27`.
fn shown_fields(fields: &[(String, FieldValue)]) -> String {
    fields
        .iter()
        .map(|(name, value)| format!("{name} {}", shown_value(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    const HOME: &str = "/data/pace";

    fn line(change: &SetupChange) -> String {
        change_line(change, Path::new(HOME))
    }

    #[test]
    fn changed_setting_shows_both_values() {
        let change = SetupChange::replacing(
            FieldEdit::set(
                "/config/pace.global.toml",
                &["time-zone"],
                FieldValue::String("Europe/Vienna".to_owned()),
            ),
            "Europe/Berlin",
        );

        assert_eq!(
            line(&change),
            "pace.global.toml: time-zone Europe/Berlin → Europe/Vienna"
        );
    }

    #[test]
    fn new_field_in_home_shows_path_in_home() {
        let change = SetupChange::added(FieldEdit::set(
            "/data/pace/projects/webshop.toml",
            &["billable"],
            FieldValue::Boolean(true),
        ));

        assert_eq!(line(&change), "projects/webshop.toml: billable → true");
    }

    #[test]
    fn appended_rate_shows_its_fields() {
        let change = SetupChange::added(FieldEdit::append_table(
            "/data/pace/clients.toml",
            &["acme", "rates"],
            vec![
                (
                    "hourly".to_owned(),
                    FieldValue::Decimal("105.00".to_owned()),
                ),
                ("from".to_owned(), FieldValue::Date(date(2026, 12, 1))),
            ],
        ));

        assert_eq!(
            line(&change),
            "clients.toml: acme.rates + hourly 105.00, from 2026-12-01"
        );
    }

    #[test]
    fn removed_field_says_removed() {
        let change = SetupChange::replacing(
            FieldEdit::remove("/data/pace/projects/webshop.toml", &["client"]),
            "acme",
        );

        assert_eq!(
            line(&change),
            "projects/webshop.toml: client acme → removed"
        );
    }
}
