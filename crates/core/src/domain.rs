//! Domain models and business rules

pub mod activity;
pub mod category;
pub mod filter;
pub mod inbox;
pub mod intermission;
pub mod priority;
pub mod project;
pub mod review;
pub mod status;
pub mod tag;
pub mod task;
pub mod time;

struct Session {
    id: usize,
    task_id: usize,
    start_time: u64,       // Unix timestamp
    end_time: Option<u64>, // Unix timestamp
}

struct TimeEntry {
    task_id: usize,
    start_time: u64,       // Unix timestamp
    end_time: Option<u64>, // Unix timestamp
}

struct Context {
    id: usize,
    name: String,
    description: Option<String>,
}
