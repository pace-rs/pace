use chrono_tz::Tz;
#[cfg(feature = "clap")]
use clap::Parser;
use getset::{Getters, MutGetters, Setters};
use pace_time::{
    flags::{DateFlags, TimeFlags},
    time_frame::PaceTimeFrame,
};
use serde_derive::Serialize;
use std::path::PathBuf;
use tracing::debug;
use typed_builder::TypedBuilder;

use crate::{
    config::PaceConfig,
    domain::{activity::ActivityKind, filter::FilterOptions, reflection::ReflectionsFormatKind},
    error::{PaceResult, UserMessage},
    service::{activity_store::ActivityStore, activity_tracker::ActivityTracker},
    storage::get_storage_from_config,
};

/// `reflect` subcommand options
#[derive(Debug, Getters)]
#[getset(get = "pub")]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(
        feature = "clap", clap(group = clap::ArgGroup::new("tz").multiple(false).required(false)))]
pub struct ReflectCommandOptions {
    /// Filter by activity kind (e.g., activity, task)
    #[cfg_attr(
        feature = "clap",
        clap(short, long, value_name = "Activity Kind", visible_alias = "kind")
    )]
    activity_kind: Option<ActivityKind>,

    /// Filter by category name, wildcard supported
    #[cfg_attr(
        feature = "clap",
        clap(short, long, value_name = "Category", visible_alias = "cat")
    )]
    category: Option<String>,

    /// Case sensitive category filter
    #[cfg_attr(feature = "clap", clap(long, value_name = "Case Sensitive"))]
    case_sensitive: bool,

    /// Specify output format (e.g., text, markdown, pdf)
    #[cfg_attr(
        feature = "clap",
        clap(short, long, value_name = "Output Format", visible_alias = "format")
    )]
    output_format: Option<ReflectionsFormatKind>,

    /// Export the reflections to a specified file
    #[cfg_attr(
        feature = "clap",
        clap(short, long, value_name = "Export File", visible_alias = "export")
    )]
    export_file: Option<PathBuf>,

    /// Time flags
    #[cfg_attr(
        feature = "clap",
        clap(flatten, next_help_heading = "Flags for specifying time periods")
    )]
    time_flags: TimeFlags,

    /// Date flags
    #[cfg_attr(
        feature = "clap",
        clap(
            flatten,
            next_help_heading = "Date flags for specifying custom date ranges or specific dates"
        )
    )]
    date_flags: DateFlags,

    /// Time zone to use for the activity, e.g., "Europe/Amsterdam"
    #[cfg_attr(
        feature = "clap",
        clap(long, value_name = "Time Zone", group = "tz", visible_alias = "tz")
    )]
    time_zone: Option<Tz>,

    /// Time zone offset to use for the activity, e.g., "+0200" or "-0500". Format: ±HHMM
    #[cfg_attr(
        feature = "clap",
        clap(
            long,
            value_name = "Time Zone Offset",
            group = "tz",
            visible_alias = "tzo"
        )
    )]
    time_zone_offset: Option<String>,

    /// Expensive flags
    /// These flags are expensive to compute and may take longer to generate
    #[cfg_attr(
        feature = "clap",
        clap(flatten, next_help_heading = "Expensive flags for detailed insights")
    )]
    expensive_flags: ExpensiveFlags,
}

impl ReflectCommandOptions {
    #[tracing::instrument(skip(self))]
    pub fn handle_reflect(&self, config: &PaceConfig) -> PaceResult<UserMessage> {
        let Self {
            export_file,
            time_flags,
            date_flags,
            time_zone,
            time_zone_offset,
            .. // TODO: ignore the rest of the fields for now,
        } = self;

        // Validate the time and time zone as early as possible
        let time_frame = PaceTimeFrame::try_from((
            time_flags,
            date_flags,
            time_zone
                .as_ref()
                .or_else(|| config.general().default_time_zone().as_ref()),
            time_zone_offset.as_ref(),
        ))?;

        let activity_store = ActivityStore::with_storage(get_storage_from_config(config)?)?;

        let activity_tracker = ActivityTracker::with_activity_store(activity_store);

        debug!("Displaying reflection for time frame: {}", time_frame);

        let Some(reflections) =
            activity_tracker.generate_reflection(FilterOptions::from(self), time_frame)?
        else {
            return Ok(UserMessage::new(
                "No activities found for the specified time frame",
            ));
        };

        match self.output_format() {
            Some(ReflectionsFormatKind::Console) | None => {
                return Ok(UserMessage::new(reflections.to_string()));
            }
            Some(ReflectionsFormatKind::Json) => {
                let json = serde_json::to_string_pretty(&reflections)?;

                debug!("Reflection: {}", json);

                // write to file if export file is specified
                if let Some(export_file) = export_file {
                    std::fs::write(export_file, json)?;

                    return Ok(UserMessage::new(format!(
                        "Reflection generated: {}",
                        export_file.display()
                    )));
                }

                return Ok(UserMessage::new(json));
            }

            Some(ReflectionsFormatKind::Html) => unimplemented!("HTML format not yet supported"),
            Some(ReflectionsFormatKind::Csv) => unimplemented!("CSV format not yet supported"),
            Some(ReflectionsFormatKind::Markdown) => {
                unimplemented!("Markdown format not yet supported")
            }
            Some(ReflectionsFormatKind::PlainText) => {
                unimplemented!("Plain text format not yet supported")
            }
        }
    }
}

#[derive(
    Debug, TypedBuilder, Serialize, Getters, Setters, MutGetters, Clone, Eq, PartialEq, Default,
)]
#[cfg_attr(feature = "clap", derive(Parser))]
pub struct ExpensiveFlags {
    /// Include detailed time logs in the reflection
    #[cfg_attr(feature = "clap", clap(long))]
    detailed: bool,

    /// Enable comparative insights against a previous period
    #[cfg_attr(feature = "clap", clap(long))]
    comparative: bool,

    /// Enable personalized recommendations based on reflection data
    #[cfg_attr(feature = "clap", clap(long))]
    recommendations: bool,
}
