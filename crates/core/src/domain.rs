//! Domain models and business rules

/// The kind of activity a user can track
pub mod activity;

/// A log file  of activities
pub mod activity_log;

/// A category for activities
pub mod category;

/// A filter for activities
pub mod filter;
pub mod inbox;
pub mod intermission;
pub mod priority;
pub mod project;
pub mod reflection;
pub mod status;
pub mod tag;
pub mod task;

struct Session {
    id: usize,
    task_id: usize,
    start_time: u64,       // Unix timestamp
    end_time: Option<u64>, // Unix timestamp
}

struct Context {
    id: usize,
    name: String,
    description: Option<String>,
}
