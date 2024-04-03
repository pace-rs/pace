#[cfg(feature = "clap")]
use clap::Parser;
use getset::{Getters, MutGetters, Setters};
use serde_derive::Serialize;
use std::{path::PathBuf, sync::Arc};
use tracing::debug;
use typed_builder::TypedBuilder;

use pace_core::{
    config::PaceConfig,
    domain::{activity::ActivityKind, category::PaceCategory, reflection::ReflectionsFormatKind},
    options::FilterOptions,
    storage::ActivityStorage,
    template::{PaceReflectionTemplate, TEMPLATES},
};
use pace_error::{PaceResult, TemplatingErrorKind, UserMessage};
use pace_service::{activity_store::ActivityStore, activity_tracker::ActivityTracker};
use pace_time::{
    flags::{DateFlags, TimeFlags},
    time_frame::PaceTimeFrame,
    time_zone::PaceTimeZoneKind,
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
    category: Option<PaceCategory>,

    /// Case sensitive category filter
    #[cfg_attr(
        feature = "clap",
        clap(short = 'i', long, value_name = "Case Sensitive")
    )]
    case_sensitive: bool,

    /// Specify output format for the reflection
    #[cfg_attr(
        feature = "clap",
        clap(short, long, value_name = "Output Format", visible_alias = "format",)
    )]
    output_format: Option<ReflectionsFormatKind>,

    /// Use this template for rendering the reflection
    // TODO: Make it dependent on the `output_format` argument
    #[cfg_attr(
        feature = "clap",
        clap(short, long, value_name = "Template File", visible_alias = "tpl")
    )]
    template_file: Option<PathBuf>,

    /// Export the reflections to a specified file
    #[cfg_attr(
        feature = "clap",
        clap(short, long, value_name = "Export File", visible_alias = "export")
    )]
    export_file: Option<PathBuf>,

    /// Time flags
    #[cfg_attr(
        feature = "clap",
        clap(
            rename_all = "kebab-case",
            value_name = "Time Flags",
            next_help_heading = "Flags for specifying time periods"
        )
    )]
    time_flags: Option<TimeFlags>,

    /// Date flags
    #[cfg_attr(
        feature = "clap",
        clap(
            flatten,
            next_help_heading = "Date flags for specifying custom date ranges or specific dates"
        )
    )]
    date_flags: Option<DateFlags>,

    // TODO! Implement time zone and time zone offset support, does it make sense?
    // - [ ] Determine how we should implement it, how does a potential user want to use this?
    // - [ ] for now showing the reflection in the local time zone is good enough
    // /// Time zone to use for displaying the reflections, e.g., "Europe/Amsterdam"
    // #[cfg_attr(
    //     feature = "clap",
    //     clap(long, value_name = "Time Zone", group = "tz", visible_alias = "tz")
    // )]
    // time_zone: Option<Tz>,

    // /// Time zone offset to use to display the reflections, e.g., "+0200" or "-0500". Format: ±HHMM
    // #[cfg_attr(
    //     feature = "clap",
    //     clap(
    //         long,
    //         value_name = "Time Zone Offset",
    //         group = "tz",
    //         visible_alias = "tzo"
    //     )
    // )]
    // time_zone_offset: Option<FixedOffset>,
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
    pub fn handle_reflect(
        &self,
        config: &PaceConfig,
        storage: Arc<dyn ActivityStorage>,
    ) -> PaceResult<UserMessage> {
        let Self {
            export_file,
            time_flags,
            date_flags,
            template_file,
            output_format,
            category,
            case_sensitive,
            // time_zone,
            // time_zone_offset,
            .. // TODO: ignore the rest of the fields for now,
        } = self;

        // Validate the time and time zone as early as possible
        let time_frame = PaceTimeFrame::try_from((
            time_flags.as_ref(),
            date_flags.as_ref(),
            PaceTimeZoneKind::NotSet,
            PaceTimeZoneKind::NotSet,
        ))?;

        let activity_store = ActivityStore::with_storage(storage)?;

        let activity_tracker = ActivityTracker::with_activity_store(activity_store);

        debug!("Displaying reflection for time frame: {time_frame}");

        let filter_opts = FilterOptions::builder()
            .category(category.clone())
            .case_sensitive(*case_sensitive)
            .build();

        let Some(reflection) = activity_tracker.generate_reflection(filter_opts, time_frame)?
        else {
            return Ok(UserMessage::new(
                "No activities found for the specified time frame",
            ));
        };

        match output_format {
            Some(ReflectionsFormatKind::Console) | None => {
                return Ok(UserMessage::new(reflection.to_string()));
            }
            Some(ReflectionsFormatKind::Json) => {
                let json = serde_json::to_string_pretty(&reflection)?;

                debug!("Reflection: {json}");

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
            Some(ReflectionsFormatKind::Template) => {
                let context = PaceReflectionTemplate::from(reflection).into_context();

                let templated = if template_file.is_none() {
                    TEMPLATES
                        .render("base.html", &context)
                        .map_err(TemplatingErrorKind::RenderingToTemplateFailed)?
                } else {
                    let Some(user_tpl) = template_file.as_ref() else {
                        return Err(TemplatingErrorKind::TemplateFileNotSpecified.into());
                    };

                    let user_tpl = std::fs::read_to_string(user_tpl)
                        .map_err(TemplatingErrorKind::FailedToReadTemplateFile)?;

                    tera::Tera::one_off(&user_tpl, &context, true)
                        .map_err(TemplatingErrorKind::RenderingToTemplateFailed)?
                };

                debug!("Reflection: {templated}");

                // write to file if export file is specified
                if let Some(export_file) = export_file {
                    std::fs::write(export_file, templated)?;

                    return Ok(UserMessage::new(format!(
                        "Reflection generated: {}",
                        export_file.display()
                    )));
                }

                return Ok(UserMessage::new(templated));
            }
            Some(ReflectionsFormatKind::Csv) => unimplemented!("CSV format not yet supported"),
            _ => unimplemented!("Unsupported output format"),
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
