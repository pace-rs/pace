# pace

pace is a timetracking application for the command-line.

## Command Overview

### Key

❌ = not implemented, yet

⏲️ = work in progress

✅ = fully functioning

### Commands

❌ **`pace begin <task-name>`**

- **Description:** Starts tracking time for the specified task. You can
  optionally specify a category or project to help organize your tasks.
- **Usage:** `pace begin "Design Work" --category "Freelance"`

❌ **`pace hold <task-name>`**

- **Description:** Pauses the time tracking for the specified task. This is
  useful for taking breaks without ending the task.
- **Usage:** `pace hold "Design Work"`

❌ **`pace resume <task-name>`**

- **Description:** Resumes time tracking for a previously paused task, allowing
  you to continue where you left off.
- **Usage:** `pace resume "Design Work"`

❌ **`pace end <task-name>`**

- **Description:** Stops time tracking for the specified task, marking it as
  completed or finished for the day.
- **Usage:** `pace end "Design Work"`

❌ **`pace tasks`**

- **Description:** Lists all tasks with optional filters. Use this to view
  active, completed, or today's tasks.
- **Usage:** `pace tasks --active`

❌ **`pace now`**

- **Description:** Displays the currently running task, showing you at a glance
  what you're currently tracking.
- **Usage:** `pace now`

❌ **`pace pomo <task-name>`**

- **Description:** Starts a Pomodoro session for the specified task, integrating
  the Pomodoro technique directly with your tasks.
- **Usage:** `pace pomo "Study Session"`

❌ **`pace report --daily/--weekly/--monthly`**

- **Description:** Generates a report for your tasks. You can specify the time
  frame for daily, weekly, or monthly reports.
- **Usage:** `pace report --weekly --summary`

❌ **`pace export --json/--csv`**

- **Description:** Exports your tracked data and reports in JSON or CSV format,
  suitable for analysis or record-keeping.
- **Usage:** `pace export --csv --from 2021-01-01 --to 2021-01-31`

❌ **`pace set`**

- **Description:** Sets various application configurations, including Pomodoro
  lengths and preferred report formats.
- **Usage:** `pace set --work 25 --break 5`

❌ **`pace help`**

- **Description:** Displays help information, offering quick access to command
  usage and options.
- **Usage:** `pace help` or `pace <command> --help`
