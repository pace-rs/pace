//! Activity entity and business logic

use chrono::{Local, NaiveDateTime};
use core::fmt::Formatter;
use getset::{Getters, MutGetters, Setters};
use merge::Merge;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;
use typed_builder::TypedBuilder;
use ulid::Ulid;

use crate::domain::{
    intermission::IntermissionPeriod,
    time::{duration_to_str, BeginDateTime, PaceDuration},
};

/// The kind of activity a user can track
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash, Copy)]
#[serde(rename_all = "snake_case")]
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

    /// The end date and time of the activity
    #[builder(default, setter(strip_option))]
    #[getset(get = "pub", get_mut = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<NaiveDateTime>,

    /// The start date and time of the activity
    #[getset(get = "pub")]
    #[builder(default)]
    #[merge(skip)]
    begin: BeginDateTime,

    /// The duration of the activity
    #[builder(default, setter(strip_option))]
    #[getset(get = "pub", get_mut = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    duration: Option<PaceDuration>,

    /// The kind of activity
    #[builder(default)]
    #[merge(skip)]
    kind: ActivityKind,

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
    pomodoro_cycle: Option<PomodoroCycle>,

    // Intermission-specific attributes
    /// The intermission periods of the activity
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    intermission_periods: Option<Vec<IntermissionPeriod>>,
}

impl Default for Activity {
    fn default() -> Self {
        Self {
            guid: Some(ActivityGuid::default()),
            category: Some("Uncategorized".to_string()),
            description: Some("This is an example activity".to_string()),
            end: None,
            begin: BeginDateTime::default(),
            duration: None,
            kind: ActivityKind::Activity,
            pomodoro_cycle: None,
            intermission_periods: None,
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
    pub const fn is_active(&self) -> bool {
        self.end.is_none()
    }

    /// If the activity has ended
    #[must_use]
    pub const fn has_ended(&self) -> bool {
        self.end.is_some()
    }

    // pub fn start_intermission(&mut self, date: NaiveDate, time: NaiveTime) {
    //     let new_intermission = IntermissionPeriod::new(date, time);
    //     if let Some(ref mut periods) = self.intermission_periods {
    //         periods.push(new_intermission);
    //     } else {
    //         self.intermission_periods = Some(vec![new_intermission]);
    //     }
    // }

    // pub fn end_intermission(&mut self, date: NaiveDate, time: NaiveTime) {
    //     if let Some(intermission_periods) = &mut self.intermission_periods {
    //         if let Some(last_period) = intermission_periods.last_mut() {
    //             // Assuming intermissions can't overlap, the last one is the one to end
    //             last_period.end(date, time);
    //         }
    //     }
    // }
}
