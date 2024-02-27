//! Activity entity and business logic

use chrono::Local;
use core::fmt::Formatter;
use getset::{Getters, MutGetters, Setters};
use merge::Merge;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;
use typed_builder::TypedBuilder;
use ulid::Ulid;

use crate::{
    calculate_duration,
    domain::{
        status::ActivityStatus,
        time::{duration_to_str, PaceDateTime, PaceDuration},
    },
    PaceResult,
};

#[derive(Debug, TypedBuilder, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct ActivityItem {
    guid: ActivityGuid,
    activity: Activity,
}

impl ActivityItem {
    /// Create a new `ActivityItem`
    ///
    /// # Arguments
    ///
    /// * `guid` - The unique identifier of the activity
    /// * `activity` - The activity
    ///
    /// # Returns
    ///
    /// Returns a new `ActivityItem`
    pub fn new(guid: ActivityGuid, activity: Activity) -> Self {
        Self { guid, activity }
    }

    /// Consumes the `ActivityItem` and returns the inner `ActivityGuid` and `Activity`
    pub fn into_parts(self) -> (ActivityGuid, Activity) {
        (self.guid, self.activity)
    }
}

impl From<Activity> for ActivityItem {
    fn from(activity: Activity) -> Self {
        Self {
            guid: ActivityGuid::default(),
            activity,
        }
    }
}

impl From<(ActivityGuid, Activity)> for ActivityItem {
    fn from((guid, activity): (ActivityGuid, Activity)) -> Self {
        Self { guid, activity }
    }
}

/// The kind of activity a user can track
#[derive(
    Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash, Copy, PartialOrd, Ord,
)]
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

    pub fn to_symbol(&self) -> &'static str {
        match self {
            Self::Activity => "📆",
            Self::Task => "📋",
            Self::Intermission => "⏸️",
            Self::PomodoroWork => "🍅⏲️",
            Self::PomodoroIntermission => "🍅⏸️",
        }
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
    Debug,
    Serialize,
    Deserialize,
    TypedBuilder,
    Getters,
    Setters,
    MutGetters,
    Clone,
    Eq,
    PartialEq,
    Default,
)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
#[derive(Merge)]
pub struct Activity {
    /// The category of the activity
    // TODO: We had it as a struct before with an ID, but it's questionable if we should go for this
    // TODO: Reconsider when we implement the project management part
    // category: Category,
    #[builder(default, setter(into))]
    #[getset(get = "pub", get_mut = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[merge(strategy = crate::util::overwrite_left_with_right)]
    category: Option<String>,

    /// The description of the activity
    // This needs to be an Optional, because we use the whole activity struct
    // as well for intermissions, which don't have a description
    #[builder(setter(into))]
    #[merge(strategy = crate::util::overwrite_left_with_right)]
    description: String,

    /// The start date and time of the activity
    #[builder(default, setter(into))]
    #[getset(get = "pub")]
    // TODO: Should the begin time be updatable?
    #[merge(skip)]
    begin: PaceDateTime,

    #[builder(default)]
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub", get_mut = "pub")]
    #[merge(strategy = crate::util::overwrite_left_with_right)]
    activity_end_options: Option<ActivityEndOptions>,

    /// The kind of activity
    #[builder(default)]
    #[merge(skip)]
    kind: ActivityKind,

    /// Optional attributes for the activity kind
    #[builder(default, setter(into))]
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    #[merge(strategy = crate::util::overwrite_left_with_right)]
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
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[merge(strategy = crate::util::overwrite_left_with_right)]
    pomodoro_cycle_options: Option<PomodoroCycle>,

    #[serde(default)]
    #[builder(default)]
    #[merge(strategy = crate::util::overwrite_left_with_right)]
    status: ActivityStatus,
}

#[derive(
    Debug, Serialize, Deserialize, TypedBuilder, Getters, Setters, MutGetters, Clone, Eq, PartialEq,
)]
#[getset(get = "pub")]
pub struct ActivityEndOptions {
    /// The end date and time of the activity
    #[builder(default)]
    #[getset(get = "pub")]
    end: PaceDateTime,

    /// The duration of the activity
    #[builder(default)]
    #[getset(get = "pub")]
    duration: PaceDuration,
}

impl ActivityEndOptions {
    pub fn new(end: PaceDateTime, duration: PaceDuration) -> Self {
        Self { end, duration }
    }
}

#[derive(
    Debug,
    Serialize,
    Deserialize,
    TypedBuilder,
    Getters,
    Setters,
    MutGetters,
    Clone,
    Eq,
    PartialEq,
    Default,
)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
#[derive(Merge)]
#[serde(rename_all = "kebab-case")]
pub struct ActivityKindOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[merge(skip)]
    parent_id: Option<ActivityGuid>,
}

impl ActivityKindOptions {
    pub fn with_parent_id(parent_id: ActivityGuid) -> Self {
        Self {
            parent_id: parent_id.into(),
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

        let nop_cat = "Uncategorized".to_string();

        write!(
            f,
            "{}  Activity: \"{}\" ({}) started {}",
            self.kind.to_symbol(),
            self.description(),
            self.category().as_ref().unwrap_or(&nop_cat),
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
    /// Create a new activity from this activity to resume
    /// an already ended/archived/etc. activity
    pub fn new_from_self(&self) -> Self {
        Self::builder()
            .description(self.description.clone())
            .category(self.category.clone())
            .kind(self.kind)
            .activity_kind_options(self.activity_kind_options.clone())
            .pomodoro_cycle_options(self.pomodoro_cycle_options)
            .build()
    }

    /// If the activity is held
    pub fn is_held(&self) -> bool {
        self.status.is_held()
    }

    /// If the activity is active, so if it is currently being tracked
    /// Intermissions are not considered active activities, please use
    /// [`is_active_intermission`] for that
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.activity_end_options().is_none()
            && (!self.kind.is_intermission() || !self.kind.is_pomodoro_intermission())
            && self.status.is_active()
    }

    /// Make the activity active
    pub fn make_active(&mut self) {
        self.status = ActivityStatus::Active;
    }

    /// Make the activity inactive
    pub fn make_inactive(&mut self) {
        self.status = ActivityStatus::Inactive;
    }

    /// Archive the activity
    /// This is only possible if the activity is not active and has ended
    pub fn archive(&mut self) {
        if !self.is_active() && self.has_ended() {
            self.status = ActivityStatus::Archived;
        }
    }

    /// Unarchive the activity
    /// This is only possible if the activity is archived
    pub fn unarchive(&mut self) {
        if self.is_archived() {
            self.status = ActivityStatus::Unarchived;
        }
    }

    /// If the activity is an active intermission
    #[must_use]
    pub fn is_active_intermission(&self) -> bool {
        self.activity_end_options().is_none()
            && (self.kind.is_intermission() || self.kind.is_pomodoro_intermission())
            && self.status.is_active()
    }

    /// If the activity is archived
    #[must_use]
    pub fn is_archived(&self) -> bool {
        self.status.is_archived()
    }

    /// If the activity is inactive
    #[must_use]
    pub fn is_inactive(&self) -> bool {
        self.status.is_inactive()
    }

    /// If the activity has ended and is not archived
    #[must_use]
    pub fn has_ended(&self) -> bool {
        self.activity_end_options().is_some()
            && (!self.kind.is_intermission() || !self.kind.is_pomodoro_intermission())
            && !self.is_archived()
            && self.status.is_ended()
    }

    /// End the activity
    ///
    /// # Arguments
    ///
    /// * `end` - The end date and time of the activity
    /// * `duration` - The [`PaceDuration`] of the activity
    pub fn end_activity(&mut self, end_opts: ActivityEndOptions) {
        self.activity_end_options = Some(end_opts);
        self.status = ActivityStatus::Ended;
    }

    /// End the activity with a given end date and time
    ///
    /// # Arguments
    ///
    /// * `begin` - The begin date and time of the activity (for calculating the duration)
    /// * `end` - The end date and time of the activity
    ///
    /// # Errors
    ///
    /// Returns an error if the duration cannot be calculated
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the activity is ended successfully
    pub fn end_activity_with_duration_calc(
        &mut self,
        begin: PaceDateTime,
        end: PaceDateTime,
    ) -> PaceResult<()> {
        let end_opts = ActivityEndOptions::new(end, calculate_duration(&begin, end)?);

        self.end_activity(end_opts);

        Ok(())
    }

    /// Get `parent_id` if activity is intermission
    ///
    /// # Returns
    ///
    /// * `Some(ActivityGuid)` - The `parent_id` of the activity
    /// * `None` - If the activity is not an intermission
    #[must_use]
    pub fn parent_id(&self) -> Option<ActivityGuid> {
        self.activity_kind_options
            .as_ref()
            .and_then(|opts| opts.parent_id)
    }
}

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use chrono::NaiveDateTime;

    use super::*;

    #[test]
    fn test_parse_single_toml_activity_passes() {
        let toml = r#"
            category = "Work"
            description = "This is an example activity"
            end = "2021-08-01T12:00:00"
            begin = "2021-08-01T10:00:00"
            duration = 5
            kind = "activity"
        "#;

        let activity: Activity = toml::from_str(toml).unwrap();

        assert_eq!(activity.category.as_ref().unwrap(), "Work");

        assert_eq!(activity.description, "This is an example activity");

        let ActivityEndOptions { end, duration } = activity.activity_end_options().clone().unwrap();

        assert_eq!(
            end,
            PaceDateTime::from(
                NaiveDateTime::parse_from_str("2021-08-01T12:00:00", "%Y-%m-%dT%H:%M:%S").unwrap()
            )
        );

        assert_eq!(
            activity.begin,
            PaceDateTime::from(
                NaiveDateTime::parse_from_str("2021-08-01T10:00:00", "%Y-%m-%dT%H:%M:%S").unwrap()
            )
        );

        assert_eq!(duration, PaceDuration::from_str("5").unwrap());

        assert_eq!(activity.kind, ActivityKind::Activity);
    }

    #[test]
    fn test_parse_single_toml_intermission_passes() {
        let toml = r#"
            end = "2021-08-01T12:00:00"
            begin = "2021-08-01T10:00:00"
            description = "This is an example activity"
            duration = 50
            kind = "intermission"
            parent-id = "01F9Z4Z3Z3Z3Z4Z3Z3Z3Z3Z3Z4" 
        "#;

        let activity: Activity = toml::from_str(toml).unwrap();

        let ActivityEndOptions { end, duration } = activity.activity_end_options().clone().unwrap();

        assert_eq!(
            end,
            PaceDateTime::from(
                NaiveDateTime::parse_from_str("2021-08-01T12:00:00", "%Y-%m-%dT%H:%M:%S").unwrap()
            )
        );

        assert_eq!(duration, PaceDuration::from_str("50").unwrap());

        assert_eq!(
            activity.begin,
            PaceDateTime::from(
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
