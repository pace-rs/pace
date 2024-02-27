//! # Pace Core

pub(crate) mod commands;
pub(crate) mod config;
pub(crate) mod domain;
pub(crate) mod error;
pub(crate) mod service;
pub(crate) mod storage;
pub(crate) mod util;

pub const PACE_CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PACE_CONFIG_FILENAME: &str = "pace.toml";
pub const PACE_ACTIVITY_LOG_FILENAME: &str = "activities.pace.toml";

// Re-export commonly used external crates
pub use toml;

// Public API
pub use crate::{
    commands::{
        hold::HoldOptions, resume::ResumeOptions, DeleteOptions, EndOptions, KeywordOptions,
        UpdateOptions,
    },
    config::{
        find_root_config_file_path, find_root_project_file, get_activity_log_paths,
        get_config_paths, get_home_activity_log_path, get_home_config_path, AutoArchivalConfig,
        DatabaseConfig, ExportConfig, GeneralConfig, InboxConfig, PaceConfig, PomodoroConfig,
        ReviewConfig,
    },
    domain::{
        activity::{
            Activity, ActivityEndOptions, ActivityGuid, ActivityItem, ActivityKind,
            ActivityKindOptions,
        },
        activity_log::ActivityLog,
        filter::{ActivityStatusFilter, FilteredActivities},
        intermission::IntermissionAction,
        status::ActivityStatus,
        time::{
            calculate_duration, duration_to_str, extract_time_or_now, parse_time_from_user_input,
            PaceDate, PaceDateTime, PaceDuration, PaceDurationRange, PaceTime, TimeRangeOptions,
        },
    },
    error::{PaceError, PaceErrorKind, PaceOptResult, PaceResult, TestResult},
    service::activity_store::ActivityStore,
    storage::{
        file::TomlActivityStorage, get_storage_from_config, in_memory::InMemoryActivityStorage,
        ActivityQuerying, ActivityReadOps, ActivityStateManagement, ActivityStorage,
        ActivityWriteOps, SyncStorage,
    },
    util::overwrite_left_with_right,
};
