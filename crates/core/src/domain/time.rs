use chrono::{DateTime, Local};

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

/// Converts Timespec to nice readable relative time string
pub fn duration_to_str(init: DateTime<Local>) -> String {
    let now = Local::now();
    let delta = now.signed_duration_since(init);

    let delta = (
        delta.num_days(),
        delta.num_hours(),
        delta.num_minutes(),
        delta.num_seconds(),
    );

    match delta {
        (days, ..) if days > 5 => format!("{}", init.format("%b %d, %Y")),
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
