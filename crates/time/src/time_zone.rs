use chrono::Local;

/// Get the local time zone offset to UTC to guess the time zones
///
/// # Returns
///
/// The local time zone offset
#[must_use]
pub fn get_local_time_zone_offset() -> i32 {
    Local::now().offset().local_minus_utc()
}
