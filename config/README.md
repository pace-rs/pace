<p align="center">
<img src="https://raw.githubusercontent.com/pace-rs/assets/main/logos/readme_header_config.png" style="max-width:500px; width:100%; height: auto" />
</p>

# Pace Configuration Specification

This specification covers all the available sections and attributes in the
`pace` configuration file and includes their corresponding environment variable
names. Users can customize their time tracking behavior by modifying these
attributes according to their needs.

**Note**: These values are subject to change as the project evolves. Especially
when we add e.g. SQLite support, the database section will be extended and the
general section will be updated with new options. In case your configuration
stops working, you can always generate a new configuration file with the
`pace setup config` command and edit it to your needs/update it.

**Note**: Some of these values are not yet implemented and are included here for
completeness. They will be implemented in future releases. If you use
`pace setup config` to generate a new configuration file, these values will not
be included. So you are safe to ignore them for now.

## Merge Precedence

The merge precedence for values is:

    Commandline Arguments >> Environment Variables >> Configuration File

Values parsed from the `configuration file` can be overwritten by
`environment variables`, which can be overwritten by `commandline arguments`
options. Therefore `commandline arguments` have the highest precedence.

## Sections and Attributes

# Pace Configuration Options

Pace can be customized to suit your preferences and workflow. Below is a
detailed overview of all available configuration options, their default values,
possible values, and descriptions to help you configure Pace to your liking.

## General Configuration

| Option                 | Default Value                        | Possible Values               | Description                                                | Environment Variable   |
| ---------------------- | ------------------------------------ | ----------------------------- | ---------------------------------------------------------- | ---------------------- |
| `activity_log_storage` | `"file"`                             | `"file"`, `"database"`        | Defines where to store the activity log.                   |                        |
| `activity_log_path`    | `"/path/to/your/activity.pace.toml"` | -                             | The path to the activity log file (if using file storage). | PACE_ACTIVITY_LOG_FILE |
| `activity_log_format`  | `"toml"`                             | `"toml"`, `"yaml"`            | Default format for new activity logs.                      |                        |
| `category_separator`   | `"::"`                               | -                             | The separator used for categories in the CLI.              |                        |
| `default_priority`     | `"medium"`                           | `"low"`, `"medium"`, `"high"` | Default priority for new tasks.                            |                        |

## Reviews

| Option             | Default Value              | Possible Values                            | Description                              |
| ------------------ | -------------------------- | ------------------------------------------ | ---------------------------------------- |
| `review_format`    | `"console"`                | `"console", "pdf"`, `"html"`, `"markdown"` | Format of the reviews generated by Pace. |
| `review_directory` | `"/path/to/your/reviews/"` | -                                          | Directory where reviews will be stored.  |

## Export

| Option                        | Default Value      | Possible Values | Description                                 |
| ----------------------------- | ------------------ | --------------- | ------------------------------------------- |
| `export_include_tags`         | `true`             | `true`, `false` | Whether to include tags in exports.         |
| `export_include_descriptions` | `true`             | `true`, `false` | Whether to include descriptions in exports. |
| `export_time_format`          | `"%Y-%m-%d %H:%M"` | -               | The time format used in exports.            |

## Database

| Option              | Default Value                | Possible Values | Description                                               |
| ------------------- | ---------------------------- | --------------- | --------------------------------------------------------- |
| `engine`            | `"sqlite"`                   | `"sqlite"`      | The database engine used (only SQLite supported for now). |
| `connection_string` | `"path/to/your/database.db"` | -               | The database connection string.                           |

## Pomodoro

| Option                        | Default Value | Possible Values | Description                                      |
| ----------------------------- | ------------- | --------------- | ------------------------------------------------ |
| `work_duration_minutes`       | `25`          | -               | Duration of a Pomodoro work session in minutes.  |
| `break_duration_minutes`      | `5`           | -               | Duration of a short break in minutes.            |
| `long_break_duration_minutes` | `15`          | -               | Duration of a long break in minutes.             |
| `sessions_before_long_break`  | `4`           | -               | Number of sessions before a long break is taken. |

## Inbox

| Option                    | Default Value | Possible Values               | Description                                             |
| ------------------------- | ------------- | ----------------------------- | ------------------------------------------------------- |
| `max_size`                | `100`         | -                             | Maximum number of items the inbox can hold.             |
| `default_priority`        | `"medium"`    | `"low"`, `"medium"`, `"high"` | Default priority for new tasks added to the inbox.      |
| `auto_archive_after_days` | `30`          | -                             | Number of days after which new tasks are auto-archived. |

## Auto Archival

| Option               | Default Value              | Possible Values | Description                                                            |
| -------------------- | -------------------------- | --------------- | ---------------------------------------------------------------------- |
| `enabled`            | `true`                     | `true`, `false` | Enable or disable automatic archival of completed tasks.               |
| `archive_after_days` | `90`                       | -               | Number of days after which completed tasks are automatically archived. |
| `archive_path`       | `"/path/to/your/archive/"` | -               | Path to the archival location (relevant if log storage is "file").     |

These configuration options allow you to tailor Pace to fit your workflow and
preferences, ensuring you get the most out of your time tracking experience.