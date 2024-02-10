<p align="center">
<img src="https://raw.githubusercontent.com/pace-rs/pace/main/assets/8th-logo-header.png" height="350" />
</p>
<p align="center"><b>pace is a timetracking application for the command line.</b></p>

<p align="center">
<a href="https://crates.io/crates/pace-rs"><img src="https://img.shields.io/crates/v/pace-rs.svg" /></a>
<a href="https://docs.rs/pace-rs/"><img src="https://img.shields.io/docsrs/pace-rs?style=flat&amp;labelColor=1c1d42&amp;color=4f396a&amp;logo=Rust&amp;logoColor=white" /></a>
<a href="https://raw.githubusercontent.com/pace-rs/pace/main/"><img src="https://img.shields.io/badge/license-AGPLv3+-red.svg" /></a>
<a href="https://crates.io/crates/pace-rs"><img src="https://img.shields.io/crates/d/pace-rs.svg" /></a>
<p>

## Command Overview

### Key

❌ = not implemented, yet

⏲️ = work in progress

🪧 = implemented, more testing needed

✅ = fully functioning

### Commands

**Note:** The following commands are subject to change as the project develops.
Currently they are stating the intended functionality and may not be fully
implemented yet (e.g. using activities instead of tasks).

🪧 **`pace begin <task-name>`**

- **Description:** Starts tracking time for the specified task. You can
  optionally specify a category or project to help organize your tasks.
- **Usage:** `pace begin "Design Work" --category "Freelance"`

🪧 **`pace end <task-name>`**

- **Description:** Stops time tracking for the specified task, marking it as
  completed or finished for the day.
- **Usage:** `pace end "Design Work"`

🪧 **`pace now`**

- **Description:** Displays the currently running task, showing you at a glance
  what you're currently tracking.
- **Usage:** `pace now`

❌ **`pace report --daily/--weekly/--monthly`**

- **Description:** Generates a report for your tasks. You can specify the time
  frame for daily, weekly, or monthly reports.
- **Usage:** `pace report --weekly --summary`

❌ **`pace resume <task-name>`**

- **Description:** Resumes time tracking for a previously paused task, allowing
  you to continue where you left off.
- **Usage:** `pace resume "Design Work"`

❌ **`pace hold <task-name>`**

- **Description:** Pauses the time tracking for the specified task. This is
  useful for taking breaks without ending the task.
- **Usage:** `pace hold` or `pace hold "Design Work"`

✅ **`pace help`**

- **Description:** Displays help information, offering quick access to command
  usage and options.
- **Usage:** `pace help` or `pace <command> --help`

### Additional Commands

❌ **`pace tasks`**

- **Description:** Lists all tasks with optional filters. Use this to view
  active, completed, or today's tasks.
- **Usage:** `pace tasks --active`

❌ **`pace projects`**

- **Description:** Lists all projects with optional filters. Use this to view
  all projects, subprojects and their associated tasks.
- **Usage:** `pace projects`

❌ **`pace pomo <task-name>`**

- **Description:** Starts a Pomodoro session for the specified task, integrating
  the Pomodoro technique directly with your tasks.
- **Usage:** `pace pomo "Study Session"`

❌ **`pace export --json/--csv`**

- **Description:** Exports your tracked data and reports in JSON or CSV format,
  suitable for analysis or record-keeping.
- **Usage:** `pace export --csv --from 2021-01-01 --to 2021-01-31`

❌ **`pace set`**

- **Description:** Sets various application configurations, including Pomodoro
  lengths and preferred report formats.
- **Usage:** `pace set --work 25 --break 5`

## License

**AGPL-3.0-or-later**; see [LICENSE](./LICENSE).
