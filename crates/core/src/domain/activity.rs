//! Activity entity and business logic

use chrono::Local;
use core::fmt::Formatter;
use getset::{Getters, MutGetters, Setters};
use merge::Merge;
use pace_time::{
    date_time::PaceDateTime,
    duration::{calculate_duration, duration_to_str, PaceDuration},
};
use strum::EnumIter;

use serde_derive::{Deserialize, Serialize};
use std::{collections::HashSet, fmt::Display};
use strum_macros::EnumString;
use tracing::debug;
use typed_builder::TypedBuilder;
use ulid::Ulid;

use crate::{
    domain::status::ActivityStatusKind,
    error::{ActivityLogErrorKind, PaceResult},
};

#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
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
    #[must_use]
    pub const fn new(guid: ActivityGuid, activity: Activity) -> Self {
        Self { guid, activity }
    }

    /// Consumes the `ActivityItem` and returns the inner `ActivityGuid` and `Activity`
    #[must_use]
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
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Default,
    PartialEq,
    Eq,
    Hash,
    Copy,
    PartialOrd,
    Ord,
    EnumString,
    EnumIter,
    strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
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

#[allow(clippy::trivially_copy_pass_by_ref)]
impl ActivityKind {
    /// Returns `true` if the activity kind is [`Activity`].
    ///
    /// [`Activity`]: ActivityKind::Activity
    #[must_use]
    pub const fn is_activity(&self) -> bool {
        matches!(self, Self::Activity)
    }

    /// Returns `true` if the activity kind is [`Task`].
    ///
    /// [`Task`]: ActivityKind::Task
    #[must_use]
    pub const fn is_task(&self) -> bool {
        matches!(self, Self::Task)
    }

    /// Returns `true` if the activity kind is [`Intermission`].
    ///
    /// [`Intermission`]: ActivityKind::Intermission
    #[must_use]
    pub const fn is_intermission(&self) -> bool {
        matches!(self, Self::Intermission)
    }

    /// Returns `true` if the activity kind is [`PomodoroWork`].
    ///
    /// [`PomodoroWork`]: ActivityKind::PomodoroWork
    #[must_use]
    pub const fn is_pomodoro_work(&self) -> bool {
        matches!(self, Self::PomodoroWork)
    }

    /// Returns `true` if the activity kind is [`PomodoroIntermission`].
    ///
    /// [`PomodoroIntermission`]: ActivityKind::PomodoroIntermission
    #[must_use]
    pub const fn is_pomodoro_intermission(&self) -> bool {
        matches!(self, Self::PomodoroIntermission)
    }

    /// Returns the symbol for the activity kind
    #[must_use]
    pub const fn as_symbol(&self) -> &'static str {
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
// TODO: How to better support subcategories
// subcategory: Option<Category>,
/// The category of the activity
// TODO: We had it as a struct before with an ID, but it's questionable if we should go for this
// TODO: Reconsider when we implement the project management part
// category: Category,
#[allow(clippy::struct_field_names)]
pub struct Activity {
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
    #[merge(strategy = crate::util::overwrite_left_with_right)]
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

    /// Tags for the activity
    #[builder(default, setter(into))]
    #[merge(strategy = crate::util::overwrite_left_with_right)]
    tags: Option<HashSet<String>>,

    // Pomodoro-specific attributes
    /// The pomodoro cycle of the activity
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[merge(strategy = crate::util::overwrite_left_with_right)]
    pomodoro_cycle_options: Option<PomodoroCycle>,

    #[serde(default)]
    #[builder(default)]
    #[merge(strategy = crate::util::overwrite_left_with_right)]
    status: ActivityStatusKind,
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
    #[must_use]
    pub const fn new(end: PaceDateTime, duration: PaceDuration) -> Self {
        Self { end, duration }
    }

    #[must_use]
    pub const fn as_tuple(&self) -> (PaceDateTime, PaceDuration) {
        (self.end, self.duration)
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
    #[must_use]
    pub fn with_parent_id(parent_id: ActivityGuid) -> Self {
        Self {
            parent_id: parent_id.into(),
        }
    }
}

/// The unique identifier of an activity
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialEq, PartialOrd, Eq, Copy, Hash)]
pub struct ActivityGuid(Ulid);

impl ActivityGuid {
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    #[must_use]
    pub const fn with_id(id: Ulid) -> Self {
        Self(id)
    }
}

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
        let time = self.begin.and_local_timezone(&Local);
        let utc_offset = time.offset();
        let symbol = self.kind.as_symbol();
        let nop_cat = "Uncategorized".to_string();
        let description = self.description();
        let category = self.category().as_ref().unwrap_or(&nop_cat);
        let started_at = duration_to_str(time);

        write!(
            f,
            "{symbol}  Activity: \"{description}\" ({category}) started {started_at} in UTC{utc_offset}",
        )
    }
}

impl Activity {
    /// Create a new activity from this activity to resume
    /// an already ended/archived/etc. activity
    #[must_use]
    pub fn new_from_self(&self) -> Self {
        debug!(
            "Creating a new activity from the current activity: {:?}.",
            self
        );

        Self::builder()
            .description(self.description.clone())
            .category(self.category.clone())
            .kind(self.kind)
            .activity_kind_options(self.activity_kind_options.clone())
            .pomodoro_cycle_options(self.pomodoro_cycle_options)
            .tags(self.tags.clone())
            .build()
    }

    /// If the activity is held
    pub fn is_paused(&self) -> bool {
        debug!("Checking if activity is held: {:?}", self);
        self.status.is_paused()
    }

    /// If the activity is active, so if it is currently being tracked
    /// Intermissions are not considered active activities, please use
    /// [`is_active_intermission`] for that
    #[must_use]
    pub fn is_in_progress(&self) -> bool {
        debug!("Checking if activity is active: {:?}", self);
        self.activity_end_options().is_none()
            && (!self.kind.is_intermission() || !self.kind.is_pomodoro_intermission())
            && self.status.is_in_progress()
    }

    /// Make the activity active
    pub fn make_active(&mut self) {
        debug!("Making activity active: {:?}", self);
        self.status = ActivityStatusKind::InProgress;
    }

    /// Make the activity inactive
    pub fn make_inactive(&mut self) {
        debug!("Making activity inactive: {:?}", self);
        self.status = ActivityStatusKind::Created;
    }

    /// Archive the activity
    /// This is only possible if the activity is not active and has ended
    pub fn archive(&mut self) {
        if !self.is_in_progress() && self.is_completed() {
            debug!("Archiving activity: {:?}", self);
            self.status = ActivityStatusKind::Archived;
        }
    }

    /// Unarchive the activity
    /// This is only possible if the activity is archived
    pub fn unarchive(&mut self) {
        if self.is_archived() {
            debug!("Unarchiving activity: {:?}", self);
            self.status = ActivityStatusKind::Unarchived;
        }
    }

    /// If the activity is endable, meaning if it is active or held
    pub fn is_completable(&self) -> bool {
        debug!("Checking if activity is endable: {:?}", self);
        self.is_in_progress() || self.is_paused()
    }

    /// If the activity is an active intermission
    #[must_use]
    pub fn is_active_intermission(&self) -> bool {
        debug!("Checking if activity is an active intermission: {:?}", self);
        self.activity_end_options().is_none()
            && (self.kind.is_intermission() || self.kind.is_pomodoro_intermission())
            && self.status.is_in_progress()
    }

    /// If the activity is archived
    #[must_use]
    pub fn is_archived(&self) -> bool {
        debug!("Checking if activity is archived: {:?}", self);
        self.status.is_archived()
    }

    /// If the activity is inactive
    #[must_use]
    pub fn is_inactive(&self) -> bool {
        debug!("Checking if activity is inactive: {:?}", self);
        self.status.is_created()
    }

    /// If the activity has ended and is not archived
    #[must_use]
    pub fn is_completed(&self) -> bool {
        debug!("Checking if activity has ended: {:?}", self);
        self.activity_end_options().is_some()
            && (!self.kind.is_intermission() || !self.kind.is_pomodoro_intermission())
            && !self.is_archived()
            && self.status.is_completed()
    }

    /// If the activity is resumable
    #[must_use]
    pub fn is_resumable(&self) -> bool {
        debug!("Checking if activity is resumable: {:?}", self);
        self.is_inactive() || self.is_archived() || self.is_paused() || self.is_completed()
    }

    /// End the activity
    ///
    /// # Arguments
    ///
    /// * `end` - The end date and time of the activity
    /// * `duration` - The [`PaceDuration`] of the activity
    pub fn end_activity(&mut self, end_opts: ActivityEndOptions) {
        debug!("Ending activity: {:?}", self);
        self.activity_end_options = Some(end_opts);
        self.status = ActivityStatusKind::Completed;
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
        let end_opts = ActivityEndOptions::new(end, calculate_duration(&begin, &end)?);

        debug!(
            "Ending activity {} with duration calculations end_opts: {:?}",
            self, end_opts
        );

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

    /// Get the overall duration of the activity
    ///
    /// # Errors
    ///
    /// Returns an error if there are no end options found
    ///
    /// # Result
    ///
    /// Returns the duration of the activity
    pub fn duration(&self) -> PaceResult<PaceDuration> {
        let end_opts = self
            .activity_end_options()
            .clone()
            .ok_or(ActivityLogErrorKind::NoEndOptionsFound)?;

        Ok(end_opts.duration)
    }
}

#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub")]
pub struct ActivitySession {
    /// A description of the activity group
    description: String,

    /// Root Activity within the activity group
    root_activity: ActivityItem,

    /// Duration spent on the grouped activities, essentially the sum of all durations
    /// of the activities within the group and their children. Intermissions are counting
    /// negatively towards the duration.
    adjusted_duration: PaceDuration,

    /// Intermissions within the activity group
    intermissions: Vec<ActivityItem>,

    /// The total duration of intermissions within the activity group
    intermission_duration: PaceDuration,
}

// TODO: Essentially a root activity and all intermissions should always have a duration, but we should
// TODO: handle the case where it doesn't.
impl ActivitySession {
    pub fn new(root_activity: ActivityItem) -> Self {
        debug!("Creating new activity session");

        debug!("Root Activity: {:#?}", root_activity.activity());

        Self {
            description: root_activity.activity().description().to_owned(),
            adjusted_duration: root_activity.activity().duration().unwrap_or_default(),
            root_activity,
            ..Default::default()
        }
    }

    pub fn add_intermission(&mut self, intermission: ActivityItem) {
        debug!("Adding intermission to activity session");

        debug!("Intermission: {:#?}", intermission.activity());

        self.intermission_duration += intermission.activity().duration().unwrap_or_default();
        self.adjusted_duration -= intermission.activity().duration().unwrap_or_default();
        self.intermissions.push(intermission);
    }

    pub fn add_multiple_intermissions(&mut self, intermissions: Vec<ActivityItem>) {
        debug!("Adding multiple intermissions to activity session");

        for intermission in intermissions {
            self.add_intermission(intermission);
        }
    }
}

/// A group of activities, the root activity and its intermissions.
#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[getset(get = "pub")]
pub struct ActivityGroup {
    /// A description of the activity group
    description: String,

    /// Duration spent on the grouped activities, essentially the sum of all durations
    /// of the activities within the group and their children. Intermissions are counting
    /// negatively towards the duration.
    adjusted_duration: PaceDuration,

    /// The total duration of intermissions within the activity group
    intermission_duration: PaceDuration,

    /// The amount of intermissions within the activity group
    intermission_count: usize,

    /// Activity sessions within the activity group
    activity_sessions: Vec<ActivitySession>,
}

impl ActivityGroup {
    pub fn with_session(activity_session: &ActivitySession) -> Self {
        debug!("Creating new activity group");

        debug!("Activity Session: {activity_session:#?}",);

        Self {
            description: activity_session.description().to_owned(),
            adjusted_duration: *activity_session.adjusted_duration(),
            intermission_count: activity_session.intermissions().len(),
            intermission_duration: *activity_session.intermission_duration(),
            ..Default::default()
        }
    }

    pub fn with_multiple_sessions(
        description: String,
        activity_sessions: Vec<ActivitySession>,
    ) -> Self {
        debug!("Creating new activity group");

        debug!("Activity Sessions: {activity_sessions:#?}",);

        let mut adjusted_duration = PaceDuration::default();
        let mut intermission_duration = PaceDuration::default();
        let mut intermission_count = 0;

        for session in &activity_sessions {
            adjusted_duration += *session.adjusted_duration();
            intermission_duration += *session.intermission_duration();
            intermission_count += session.intermissions().len();
        }

        Self {
            description,
            adjusted_duration,
            intermission_duration,
            intermission_count,
            activity_sessions,
        }
    }

    pub fn add_session(&mut self, session: ActivitySession) {
        debug!("Adding session to activity session");

        debug!("Session: {:#?}", session);

        self.intermission_duration += *session.intermission_duration();
        self.adjusted_duration -= *session.adjusted_duration();
        self.activity_sessions.push(session);
    }

    pub fn add_multiple_sessions(&mut self, sessions: Vec<ActivitySession>) {
        debug!("Adding multiple intermissions to activity session");

        for session in sessions {
            self.add_session(session);
        }
    }
}

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use chrono::{FixedOffset, NaiveDate};
    use eyre::{eyre, OptionExt};
    use pace_time::time_zone::PaceTimeZoneKind;

    use crate::error::TestResult;

    use super::*;

    #[test]
    fn test_parse_single_toml_activity_passes() -> TestResult<()> {
        let toml = r#"
            category = "Work"
            description = "This is an example activity"
            begin = "2024-03-22T14:05:14+01:00"
            end = "2024-03-22T14:05:33+01:00"
            duration = 19
            kind = "activity"
        "#;

        let activity: Activity = toml::from_str(toml)?;

        assert_eq!(activity.category.as_ref().ok_or("No category.")?, "Work");

        assert_eq!(activity.description, "This is an example activity");

        let ActivityEndOptions { end, duration } = activity
            .activity_end_options()
            .clone()
            .ok_or("No end options")?;

        let begin_time = PaceDateTime::try_from((
            NaiveDate::from_ymd_opt(2024, 3, 22)
                .ok_or_eyre("Constructing from ymd failed.")?
                .and_hms_opt(14, 5, 14)
                .ok_or_eyre("Constructing from hms failed.")?,
            PaceTimeZoneKind::TimeZoneOffset(
                FixedOffset::east_opt(3600).ok_or(eyre!("Constructing Fixed Offset failed."))?,
            ),
        ))?;

        let end_time = PaceDateTime::try_from((
            NaiveDate::from_ymd_opt(2024, 3, 22)
                .ok_or_eyre("Constructing from ymd failed.")?
                .and_hms_opt(14, 5, 33)
                .ok_or_eyre("Constructing from hms failed.")?,
            PaceTimeZoneKind::TimeZoneOffset(
                FixedOffset::east_opt(3600).ok_or_eyre("Constructing Fixed Offset failed.")?,
            ),
        ))?;

        assert_eq!(activity.begin, begin_time);

        assert_eq!(end, end_time);

        assert_eq!(duration, PaceDuration::from_str("19")?);

        assert_eq!(activity.kind, ActivityKind::Activity);

        Ok(())
    }

    #[test]
    fn test_parse_single_toml_intermission_passes() -> TestResult<()> {
        let toml = r#"
            end = "2021-08-01T12:00:00+01:00"
            begin = "2021-08-01T10:00:00+01:00"
            description = "This is an example activity"
            duration = 50
            kind = "intermission"
            parent-id = "01F9Z4Z3Z3Z3Z4Z3Z3Z3Z3Z3Z4" 
        "#;

        let activity: Activity = toml::from_str(toml)?;

        let ActivityEndOptions { end, duration } = activity
            .activity_end_options()
            .clone()
            .ok_or("No end options")?;

        let begin_time = PaceDateTime::try_from((
            NaiveDate::from_ymd_opt(2021, 8, 1)
                .ok_or_eyre("Constructing from ymd failed.")?
                .and_hms_opt(10, 0, 0)
                .ok_or_eyre("Constructing from hms failed.")?,
            PaceTimeZoneKind::TimeZoneOffset(
                FixedOffset::east_opt(3600).ok_or(eyre!("Constructing Fixed Offset failed."))?,
            ),
        ))?;

        let end_time = PaceDateTime::try_from((
            NaiveDate::from_ymd_opt(2021, 8, 1)
                .ok_or_eyre("Constructing from ymd failed.")?
                .and_hms_opt(12, 0, 0)
                .ok_or_eyre("Constructing from hms failed.")?,
            PaceTimeZoneKind::TimeZoneOffset(
                FixedOffset::east_opt(3600).ok_or_eyre("Constructing Fixed Offset failed.")?,
            ),
        ))?;

        assert_eq!(end, end_time);

        assert_eq!(activity.begin, begin_time);

        assert_eq!(duration, PaceDuration::new(50));

        assert_eq!(activity.kind, ActivityKind::Intermission);

        assert_eq!(
            activity
                .activity_kind_options
                .ok_or("No activity kind options")?
                .parent_id
                .ok_or("No parent id")?
                .to_string(),
            "01F9Z4Z3Z3Z3Z4Z3Z3Z3Z3Z3Z4"
        );

        Ok(())
    }
}
