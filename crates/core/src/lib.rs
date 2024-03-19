//! # Pace Core

pub(crate) mod commands;
pub(crate) mod config;
pub(crate) mod domain;
pub(crate) mod error;
pub(crate) mod service;
pub(crate) mod storage;
pub(crate) mod util;

// Constants
pub mod constants {
    pub const PACE_APP_NAME: &str = "pace";
    pub const PACE_CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
    pub const PACE_CONFIG_FILENAME: &str = "pace.toml";
    pub const PACE_ACTIVITY_LOG_FILENAME: &str = "activities.pace.toml";
    pub const PACE_DOCS_URL: &str = "https://pace.cli.rs/docs";
    pub const PACE_DEV_DOCS_URL: &str = "https://pace.cli.rs/dev-docs";
    pub const PACE_CONFIG_DOCS_URL: &str =
        "https://github.com/pace-rs/pace/blob/main/config/README.md";
}

// Re-export commonly used external crates
pub use merge;
pub use toml;

pub mod prelude {
    // Public Prelude API
    pub use crate::{
        commands::{
            adjust::AdjustCommandOptions,
            begin::BeginCommandOptions,
            docs::DocsCommandOptions,
            end::EndCommandOptions,
            hold::{HoldCommandOptions, HoldOptions},
            now::NowCommandOptions,
            reflect::{DateFlags, ExpensiveFlags, ReflectCommandOptions, TimeFlags},
            resume::{ResumeCommandOptions, ResumeOptions},
            DeleteOptions, EndOptions, KeywordOptions, UpdateOptions,
        },
        config::{
            find_root_config_file_path, find_root_project_file, get_activity_log_paths,
            get_config_paths, get_home_activity_log_path, get_home_config_path,
            ActivityLogFormatKind, ActivityLogStorageKind, AutoArchivalConfig, DatabaseConfig,
            ExportConfig, GeneralConfig, InboxConfig, PaceConfig, PomodoroConfig,
            ReflectionsConfig,
        },
        domain::{
            activity::{
                Activity, ActivityEndOptions, ActivityGroup, ActivityGuid, ActivityItem,
                ActivityKind, ActivityKindOptions, ActivitySession,
            },
            activity_log::ActivityLog,
            category::split_category_by_category_separator,
            filter::{ActivityFilterKind, FilterOptions, FilteredActivities},
            intermission::IntermissionAction,
            reflection::{
                Highlights, ReflectionSummary, ReflectionsFormatKind, SummaryActivityGroup,
                SummaryCategories, SummaryGroupByCategory,
            },
            status::ActivityStatus,
            time::{
                calculate_duration, duration_to_str, extract_time_or_now,
                get_local_time_zone_offset, get_time_frame_from_flags, parse_time_from_user_input,
                PaceDate, PaceDuration, PaceDurationRange, PaceNaiveDateTime, PaceTime,
                PaceTimeFrame, TimeRangeOptions,
            },
        },
        error::{PaceError, PaceErrorKind, PaceOptResult, PaceResult, TestResult, UserMessage},
        service::{activity_store::ActivityStore, activity_tracker::ActivityTracker},
        storage::{
            file::TomlActivityStorage, get_storage_from_config, in_memory::InMemoryActivityStorage,
            ActivityQuerying, ActivityReadOps, ActivityStateManagement, ActivityStorage,
            ActivityWriteOps, StorageKind, SyncStorage,
        },
        util::overwrite_left_with_right,
    };
}
