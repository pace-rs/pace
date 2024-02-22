//! Activity entity and business logic

use chrono::{Local, NaiveDateTime};
use core::fmt::Formatter;
use getset::{Getters, MutGetters, Setters};
use merge::Merge;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;
use typed_builder::TypedBuilder;
use ulid::Ulid;

use crate::{
    calculate_duration,
    domain::time::{duration_to_str, BeginDateTime, PaceDuration},
    PaceResult,
};

/// The kind of activity a user can track
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash, Copy)]
#[serde(rename_all = "kebab-case")]
// #[serde(untagged)]
pub enum ActivityKind {
    /// A generic activity
    #[default]
    Activity,

    /// A task
    Task,

    /// A break
    Intermission,

    /// A pomodoro work session
    PomodoroWork,

    /// A pomodoro break
    PomodoroIntermission,
}

impl ActivityKind {
    /// Returns `true` if the activity kind is [`Activity`].
    ///
    /// [`Activity`]: ActivityKind::Activity
    #[must_use]
    pub fn is_activity(&self) -> bool {
        matches!(self, Self::Activity)
    }

    /// Returns `true` if the activity kind is [`Task`].
    ///
    /// [`Task`]: ActivityKind::Task
    #[must_use]
    pub fn is_task(&self) -> bool {
        matches!(self, Self::Task)
    }

    /// Returns `true` if the activity kind is [`Intermission`].
    ///
    /// [`Intermission`]: ActivityKind::Intermission
    #[must_use]
    pub fn is_intermission(&self) -> bool {
        matches!(self, Self::Intermission)
    }

    /// Returns `true` if the activity kind is [`PomodoroWork`].
    ///
    /// [`PomodoroWork`]: ActivityKind::PomodoroWork
    #[must_use]
    pub fn is_pomodoro_work(&self) -> bool {
        matches!(self, Self::PomodoroWork)
    }

    /// Returns `true` if the activity kind is [`PomodoroIntermission`].
    ///
    /// [`PomodoroIntermission`]: ActivityKind::PomodoroIntermission
    #[must_use]
    pub fn is_pomodoro_intermission(&self) -> bool {
        matches!(self, Self::PomodoroIntermission)
    }
}

/// The cycle of pomodoro activity a user can track
// TODO!: Optional: Track Pomodoro work/break cycles
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
enum PomodoroCycle {
    /// A work session
    Work(usize), // usize could represent the work session number in a sequence

    // A break
    #[default]
    Intermission,
}

/// The activity entity
///
/// The activity entity is used to store and manage an activity
#[derive(
    Debug, Serialize, Deserialize, TypedBuilder, Getters, Setters, MutGetters, Clone, Eq, PartialEq,
)]
#[getset(get = "pub")]
#[derive(Merge)]
pub struct Activity {
    /// The activity's unique identifier
    #[builder(default = Some(ActivityGuid::default()), setter(strip_option))]
    #[getset(get_copy, get_mut = "pub")]
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    guid: Option<ActivityGuid>,

    /// The category of the activity
    // TODO: We had it as a struct before with an ID, but it's questionable if we should go for this
    // TODO: Reconsider when we implement the project management part
    // category: Category,
    #[builder(default)]
    #[getset(get = "pub", get_mut = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,

    /// The description of the activity
    // This needs to be an Optional, because we use the whole activity struct
    // as well for intermissions, which don't have a description
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    /// The start date and time of the activity
    #[getset(get = "pub")]
    #[builder(default)]
    #[merge(skip)]
    begin: BeginDateTime,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    #[getset(get = "pub", get_mut = "pub")]
    activity_end_options: Option<ActivityEndOptions>,

    /// The kind of activity
    #[builder(default)]
    #[merge(skip)]
    kind: ActivityKind,

    /// Optional attributes for the activity kind
    #[builder(default, setter(strip_option))]
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    activity_kind_options: Option<ActivityKindOptions>,

    // TODO: How to better support subcategories
    // subcategory: Option<Category>,

    // TODO: Was `Tag` before, but we want to check how to better support that
    // TODO: also, we should consider using a HashSet instead of a Vec
    // TODO: also, we might want to reconsider
    // #[builder(default, setter(strip_option))]
    // tags: Option<Vec<String>>,

    // Pomodoro-specific attributes
    /// The pomodoro cycle of the activity
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pomodoro_cycle_options: Option<PomodoroCycle>,
}

#[derive(
    Debug, Serialize, Deserialize, TypedBuilder, Getters, Setters, MutGetters, Clone, Eq, PartialEq,
)]
#[getset(get = "pub")]
pub struct ActivityEndOptions {
    /// The end date and time of the activity
    #[builder(default)]
    #[getset(get = "pub")]
    end: NaiveDateTime,

    /// The duration of the activity
    #[builder(default)]
    #[getset(get = "pub")]
    duration: PaceDuration,
}

impl ActivityEndOptions {
    pub fn new(end: NaiveDateTime, duration: PaceDuration) -> Self {
        Self { end, duration }
    }
}

#[derive(
    Debug, Serialize, Deserialize, TypedBuilder, Getters, Setters, MutGetters, Clone, Eq, PartialEq,
)]
#[getset(get = "pub")]
#[derive(Merge)]
#[serde(rename_all = "kebab-case")]
pub struct ActivityKindOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<ActivityGuid>,
}

impl ActivityKindOptions {
    pub fn new(parent_id: impl Into<Option<ActivityGuid>>) -> Self {
        Self {
            parent_id: parent_id.into(),
        }
    }
}

impl Default for Activity {
    fn default() -> Self {
        Self {
            guid: Some(ActivityGuid::default()),
            category: Some("Uncategorized".to_string()),
            description: Some("This is an example activity".to_string()),
            begin: BeginDateTime::default(),
            kind: ActivityKind::Activity,
            pomodoro_cycle_options: None,
            activity_kind_options: None,
            activity_end_options: None,
        }
    }
}

/// The unique identifier of an activity
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialEq, PartialOrd, Eq, Copy, Hash)]
pub struct ActivityGuid(Ulid);

impl Display for ActivityGuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for ActivityGuid {
    fn default() -> Self {
        Self(Ulid::new())
    }
}

impl Display for Activity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let rel_time = match self.begin.and_local_timezone(Local) {
            chrono::LocalResult::Single(time) => duration_to_str(time),
            chrono::LocalResult::None | chrono::LocalResult::Ambiguous(_, _) => {
                format!("at {}", self.begin)
            }
        };

        write!(
            f,
            "Activity: \"{}\" started {}",
            self.description.as_deref().unwrap_or("No description"),
            rel_time,
        )
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for ActivityGuid {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let bytes = <[u8; 16]>::column_result(value)?;
        Ok(Self(Ulid::from(u128::from_be_bytes(bytes))))
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for ActivityGuid {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.0.to_string()))
    }
}

impl Activity {
    /// If the activity is active, so if it is currently being tracked
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.activity_end_options().is_none()
    }

    /// If the activity has ended
    #[must_use]
    pub fn has_ended(&self) -> bool {
        self.activity_end_options().is_some()
    }

    /// End the activity
    ///
    /// # Arguments
    ///
    /// * `end` - The end date and time of the activity
    /// * `duration` - The [`PaceDuration`] of the activity
    pub fn end_activity(&mut self, end_opts: ActivityEndOptions) {
        self.activity_end_options = Some(end_opts);
    }

    pub fn end_activity_with_duration_calc(
        &mut self,
        begin: BeginDateTime,
        end: NaiveDateTime,
    ) -> PaceResult<()> {
        let end_opts = ActivityEndOptions::new(end, calculate_duration(&begin, end)?);
        self.end_activity(end_opts);
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_parse_single_toml_activity_passes() {
        let toml = r#"
            id = "01F9Z3Z3Z3Z3Z3Z3Z3Z3Z3Z3Z3"
            category = "Work"
            description = "This is an example activity"
            end = "2021-08-01T12:00:00"
            begin = "2021-08-01T10:00:00"
            duration = 5
            kind = "activity"
        "#;

        let activity: Activity = toml::from_str(toml).unwrap();

        assert_eq!(
            activity.guid.unwrap().to_string(),
            "01F9Z3Z3Z3Z3Z3Z3Z3Z3Z3Z3Z3"
        );

        assert_eq!(activity.category.as_ref().unwrap(), "Work");

        assert_eq!(
            activity.description.as_ref().unwrap(),
            "This is an example activity"
        );

        let ActivityEndOptions { end, duration } = activity.activity_end_options().clone().unwrap();

        assert_eq!(
            end,
            NaiveDateTime::parse_from_str("2021-08-01T12:00:00", "%Y-%m-%dT%H:%M:%S").unwrap()
        );

        assert_eq!(
            activity.begin,
            BeginDateTime::from(
                NaiveDateTime::parse_from_str("2021-08-01T10:00:00", "%Y-%m-%dT%H:%M:%S").unwrap()
            )
        );

        assert_eq!(duration, PaceDuration::from_str("5").unwrap());

        assert_eq!(activity.kind, ActivityKind::Activity);
    }

    #[test]
    fn test_parse_single_toml_intermission_passes() {
        let toml = r#"
            id = "01F9Z3Z3Z3Z3Z3Z3Z3Z3Z3Z3Z3"
            end = "2021-08-01T12:00:00"
            begin = "2021-08-01T10:00:00"
            duration = 50
            kind = "intermission"
            parent-id = "01F9Z4Z3Z3Z3Z4Z3Z3Z3Z3Z3Z4" 
        "#;

        let activity: Activity = toml::from_str(toml).unwrap();

        assert_eq!(
            activity.guid.unwrap().to_string(),
            "01F9Z3Z3Z3Z3Z3Z3Z3Z3Z3Z3Z3"
        );

        let ActivityEndOptions { end, duration } = activity.activity_end_options().clone().unwrap();

        assert_eq!(
            end,
            NaiveDateTime::parse_from_str("2021-08-01T12:00:00", "%Y-%m-%dT%H:%M:%S").unwrap()
        );

        assert_eq!(duration, PaceDuration::from_str("50").unwrap());

        assert_eq!(
            activity.begin,
            BeginDateTime::from(
                NaiveDateTime::parse_from_str("2021-08-01T10:00:00", "%Y-%m-%dT%H:%M:%S").unwrap()
            )
        );

        assert_eq!(activity.kind, ActivityKind::Intermission);

        assert_eq!(
            activity
                .activity_kind_options
                .unwrap()
                .parent_id
                .unwrap()
                .to_string(),
            "01F9Z4Z3Z3Z3Z4Z3Z3Z3Z3Z3Z4"
        );
    }
}
