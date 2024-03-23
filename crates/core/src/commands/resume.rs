use chrono::{FixedOffset, NaiveTime};
use chrono_tz::Tz;
#[cfg(feature = "clap")]
use clap::Parser;
use getset::Getters;
use pace_time::date_time::PaceDateTime;
use typed_builder::TypedBuilder;

/// `resume` subcommand options
#[derive(Debug, Getters, TypedBuilder, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "clap", derive(Parser))]
#[getset(get = "pub")]
#[cfg_attr(
        feature = "clap", clap(group = clap::ArgGroup::new("tz").multiple(false).required(false)))]
pub struct ResumeCommandOptions {
    /// The time the activity has been resumed at. Format: HH:MM
    #[cfg_attr(
        feature = "clap",
        clap(short, long, value_name = "Resumed Time", visible_alias = "resumed")
    )]
    at: Option<NaiveTime>,

    /// Show a list of all recent activities to continue
    #[cfg_attr(feature = "clap", clap(short, long))]
    #[getset(get = "pub")]
    list: bool,

    /// Time zone to use for the activity, e.g., "Europe/Amsterdam"
    #[cfg_attr(
        feature = "clap",
        clap(
            short = 'z',
            long,
            value_name = "Time Zone",
            group = "tz",
            visible_alias = "tz"
        )
    )]
    time_zone: Option<Tz>,

    /// Time zone offset to use for the activity, e.g., "+0200" or "-0500". Format: ±HHMM
    #[cfg_attr(
        feature = "clap",
        clap(
            short = 'Z',
            long,
            value_name = "Time Zone Offset",
            group = "tz",
            visible_alias = "tzo"
        )
    )]
    time_zone_offset: Option<FixedOffset>,
}

impl ResumeCommandOptions {
    // FIXME: Inner run implementation for the resume command kept in pace-rs crate for now
    // FIXME: due to the dependency on pace-cli
}

/// Options for resuming an activity
#[derive(Debug, Clone, PartialEq, TypedBuilder, Eq, Hash, Default, Getters)]
#[getset(get = "pub")]
#[non_exhaustive]
pub struct ResumeOptions {
    /// The resume time of the intermission
    #[builder(default, setter(into))]
    resume_time: Option<PaceDateTime>,
}
