//! # Pace Core

pub mod config;
pub mod domain;
pub mod options;
pub mod storage;
pub mod template;
pub mod util;

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
        config::{
            find_root_config_file_path, find_root_project_file, get_activity_log_paths,
            get_config_paths, get_home_activity_log_path, get_home_config_path,
            ActivityLogStorageKind, AutoArchivalConfig, DatabaseEngineKind, ExportConfig,
            GeneralConfig, InboxConfig, PaceConfig, PomodoroConfig, ReflectionsConfig,
            StorageConfig,
        },
        domain::{
            activity::{
                Activity, ActivityEndOptions, ActivityGroup, ActivityItem, ActivityKind,
                ActivityKindOptions, ActivitySession,
            },
            activity_log::ActivityLog,
            category::{split_category_by_category_separator, PaceCategory},
            description::PaceDescription,
            filter::{ActivityFilterKind, FilteredActivities},
            id::{
                ActivityGuid, ActivityKindGuid, ActivityStatusGuid, CategoryGuid, DescriptionGuid,
                Guid, TagGuid,
            },
            intermission::IntermissionAction,
            reflection::{
                Highlights, ReflectionSummary, ReflectionsFormatKind, SummaryActivityGroup,
                SummaryCategories, SummaryGroupByCategory,
            },
            status::ActivityStatusKind,
            tag::{PaceTag, PaceTagCollection},
        },
        storage::{
            ActivityQuerying, ActivityReadOps, ActivityStateManagement, ActivityStorage,
            ActivityWriteOps, SyncStorage,
        },
        util::overwrite_left_with_right,
    };
}
