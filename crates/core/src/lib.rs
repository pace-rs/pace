#![forbid(unsafe_code)]
#![warn(
    // missing_docs,
    rust_2018_idioms,
    trivial_casts,
    unused_lifetimes,
    unused_qualifications
)]
#![allow(unused_imports)]
#![allow(dead_code)]

pub mod config;
pub mod domain;
pub mod error;
pub mod service;
pub mod storage;
pub mod util;

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::activity::Activity;

// fn process_activity_log(file_path: &str) {
//     let activities = parse_activity_file(file_path); // Synchronously read and parse the file
//     for activity in activities {
//         handle_activity(&activity); // Process each activity sequentially
//     }
//     // Optionally, write any updates or logs back to a file
// }

// fn parse_activity_file(file_path: &str) -> Activity {
//     // Implementation for parsing the activity file
// }

// fn handle_activity(activity: &Activity) {
//     // Implementation for handling a single activity
// }
