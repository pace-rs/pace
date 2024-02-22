//! Domain models and business rules

/// The kind of activity a user can track
pub mod activity;

/// A log file  of activities
pub mod activity_log;

/// A category for activities
pub(crate) mod category;

/// A filter for activities
pub mod filter;
pub(crate) mod inbox;
pub(crate) mod intermission;
pub(crate) mod priority;
pub(crate) mod project;
pub(crate) mod review;
pub(crate) mod status;
pub(crate) mod tag;
pub(crate) mod task;

/// Time utilities
pub mod time;

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
