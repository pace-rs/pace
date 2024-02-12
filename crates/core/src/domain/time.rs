use chrono::{DateTime, Local, NaiveDate, NaiveTime, SubsecRound};

use crate::error::PaceResult;

pub enum TimeFrame {
    Custom {
        start: DateTime<Local>,
        end: DateTime<Local>,
    },
    Daily,
    DaysInThePast(u32),
    Monthly,
    MonthsInThePast(u32),
    Weekly,
    WeeksInThePast(u32),
    Yearly,
    YearsInThePast(u32),
}

/// Converts timespec to nice readable relative time string
///
/// # Arguments
///
/// * `initial_time` - The initial time to calculate the relative time from
///
/// # Returns
///
/// A string representing the relative time from the initial time
pub fn duration_to_str(initial_time: DateTime<Local>) -> String {
    let now = Local::now();
    let delta = now.signed_duration_since(initial_time);

    let delta = (
        delta.num_days(),
        delta.num_hours(),
        delta.num_minutes(),
        delta.num_seconds(),
    );

    match delta {
        (days, ..) if days > 5 => format!("{}", initial_time.format("%b %d, %Y")),
        (days @ 2..=5, ..) => format!("{days} days ago"),
        (1, ..) => "one day ago".to_string(),

        (_, hours, ..) if hours > 1 => format!("{hours} hours ago"),
        (_, 1, ..) => "an hour ago".to_string(),

        (_, _, minutes, _) if minutes > 1 => format!("{minutes} minutes ago"),
        (_, _, 1, _) => "one minute ago".to_string(),

        (_, _, _, seconds) if seconds > 0 => format!("{seconds} seconds ago"),
        _ => "just now".to_string(),
    }
}

/// Extracts time from the given string or returns the current time
///
/// # Arguments
///
/// * `time` - The time to extract or None
///
/// # Errors
///
/// [`chrono::ParseError`] - If the time cannot be parsed
///
/// # Returns
///
/// A tuple containing the time and date
pub fn extract_time_or_now(time: &Option<String>) -> PaceResult<(NaiveTime, NaiveDate)> {
    Ok(if let Some(ref time) = time {
        (
            NaiveTime::parse_from_str(time, "%H:%M")?,
            Local::now().date_naive(),
        )
    } else {
        // if no time is given, use the current time
        (
            Local::now().time().round_subsecs(0),
            Local::now().date_naive(),
        )
    })
}
