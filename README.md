<p align="center">
<img src="https://raw.githubusercontent.com/pace-rs/assets/main/logos/readme_header.png" style="max-width:500px; width:100%; height: auto" />
</p>
<p align="center"><b>Mindful Time Tracking: Simplify Your Focus and Boost Productivity Effortlessly.</b></p>

<p align="center">
<a href="https://crates.io/crates/pace-rs"><img src="https://img.shields.io/crates/v/pace-rs.svg" /></a>
<a href="https://docs.rs/pace-rs/"><img src="https://img.shields.io/docsrs/pace-rs?style=flat&amp;labelColor=1c1d42&amp;color=4f396a&amp;logo=Rust&amp;logoColor=white" /></a>
<a href="https://raw.githubusercontent.com/pace-rs/pace/main/LICENSE"><img src="https://img.shields.io/badge/license-AGPLv3+-red.svg" /></a>
<a href="https://crates.io/crates/pace-rs"><img src="https://img.shields.io/crates/d/pace-rs.svg" /></a>
<p>

## About

`pace` is a mindful productivity tool designed to help you keep track of your
activities with ease and intention.

Born from the desire to blend simplicity with effectiveness, pace offers a
command-line interface (CLI) that encourages focused work sessions, thoughtful
reflection on task durations, and a harmonious balance between work and rest.

Whether you're a developer, a writer, or anyone who values structured time
management, pace provides the framework to log activities, review progress, and
optimize how you spend your time.

With features like the first activity wizard for onboarding new users, real-time
configuration validation (upcoming), and personalized activity logs, pace is
more than a time tracker — it's your partner in crafting a productive and
mindful routine.

⚠️ **Note:** `pace` is currently in active development and is not yet ready for
production use. Expect breaking changes and incomplete features. We encourage
you to try it out and provide feedback, but please be aware that it is not yet
stable. You can find updates to `pace` in the
[CHANGELOG](https://github.com/pace-rs/pace/blob/main/CHANGELOG.md).

## Contact

You can ask questions in the
[Discussions](https://github.com/orgs/pace-rs/discussions) or have a look at the
[FAQ](https://pace.cli.rs/docs/FAQ.html).

| Contact       | Where?                                                                                                          |
| ------------- | --------------------------------------------------------------------------------------------------------------- |
| Issue Tracker | [GitHub Issues](https://github.com/pace-rs/pace/issues/new/choose)                                              |
| Discord       | [![Discord](https://dcbadge.vercel.app/api/server/RKSWrAcYdG?style=flat-square)](https://discord.gg/RKSWrAcYdG) |
| Discussions   | [GitHub Discussions](https://github.com/orgs/pace-rs/discussions)                                               |

## Getting started

Please check our [documentation](https://pace.cli.rs/docs/getting_started.html)
for more information on how to get started.

## Installation

### From binaries

#### [cargo-binstall](https://crates.io/crates/cargo-binstall)

```bash
cargo binstall pace-rs
```

#### Windows

##### [Scoop](https://scoop.sh/)

<!-- TODO!: When official
 ```bash
scoop install pace
``` -->

```bash
scoop bucket add pace https://github.com/pace-rs/pace/
scoop install pace
```

#### MacOS

##### [Homebrew](https://brew.sh/)

You can use our tap:

```bash
brew install pace-rs/homebrew-tap/pace-rs
```

<!--
#### npm

TODO!: Check if needed and how easy it is supported with cargo-dist
-->

#### From GitHub

Check out the [releases](https://github.com/pace-rs/pace/releases).

### From source

**Beware**: This installs the latest development version, which might be
unstable.

```bash
cargo install --git https://github.com/pace-rs/pace.git pace-rs
```

### crates.io

```bash
cargo install pace-rs
```

## Usage

### Key

✅ = fully functioning

🔍 = review and testing in progress

🪧 = implemented, more testing needed

⏲️ = work in progress

📜 = design stage

❌ = not implemented, yet

💡 = idea

### Commands

**Note:** The following commands are subject to change as the project develops.
Currently they are stating the intended functionality and may not be fully
implemented yet (e.g. using activities instead of tasks).

✅ **`pace setup`**

- **Description:** Create configuration files for pace, including the main
  configuration file and any additional settings. This is useful for setting up
  pace for the first time or when you need to change your settings. You can also
  generate shell completions for your shell of choice. And generate a project
  configuration file.
- **Usage:** `pace setup config` or `pace setup completions`

✅ **`pace adjust`**

- **Description:** Adjust the current activity's start time, description,
  category, or tags. This is useful for correcting mistakes or adding more
  detail to your activities.
- **Usage:**
  `pace adjust --category <Category> --description <Description> --start <Start Time>`

✅ **`pace begin`**

- **Description:** Starts tracking time for the specified task. You can
  optionally specify a category or project to help organize your tasks.
- **Usage:** `pace begin "Design Work" --category "Freelance" --start 10:00`

✅ **`pace end`**

- **Description:** Stops time tracking for the specified task, marking it as
  completed or finished for the day.
- **Usage:** `pace end --end 11:30 --only-last`

✅ **`pace docs`**

- **Description:** Opens the (dev-)documentation in your default browser.
- **Usage:** `pace docs` or `pace docs --dev`

✅ **`pace now`**

- **Description:** Displays the currently running task, showing you at a glance
  what you're currently tracking.
- **Usage:** `pace now`

✅ **`pace hold`**

- **Description:** Pauses the time tracking for the specified task. This is
  useful for taking breaks without ending the task.
- **Usage:** `pace hold --reason <Reason>`

✅ **`pace resume`**

- **Description:** Resumes time tracking for a previously paused task, allowing
  you to continue where you left off.
- **Usage:** `pace resume` or `pace resume --list`

🔍 **`pace review`**

- **Description:** Gain insight in your activities and tasks. You can specify
  the time frame for daily, weekly, or monthly insights.

  **⚠️ NOTE**: This command is currently in review and testing. Some features may
  not be fully implemented yet.
- **Usage:** `pace review --last-week` or
  `pace review --from 2024-02-10 --to 2024-03-06` or
  `pace review --today -o json -e ./data/data.json`

✅ **`pace help`**

- **Description:** Displays help information, offering quick access to command
  usage and options.
- **Usage:** `pace help`, `pace <command> --help`, or `pace help <command>`

<details>
  <summary>Some Ideas For Additional Useful Commands</summary>

**Note:** These commands are not yet implemented and are subject change to
during their design process.

💡 **`pace export --json/--csv`**

- **Description:** Exports your tracked data and insights in JSON or CSV format,
  suitable for analysis or record-keeping.
- **Usage:** `pace export --csv --from 2021-01-01 --to 2021-01-31`

💡 **`pace tasks`**

- **Description:** Lists all tasks with optional filters. Use this to view
  active, completed, or today's tasks.
- **Usage:** `pace tasks --active`

💡 **`pace projects`**

- **Description:** Lists all projects with optional filters. Use this to view
  all projects, subprojects and their associated tasks.
- **Usage:** `pace projects`

💡 **`pace pomo`**

- **Description:** Starts a Pomodoro session for the specified task, integrating
  the Pomodoro technique directly with your tasks.
- **Usage:** `pace pomo "Study Session"`

💡 **`pace set`**

- **Description:** Sets various application configurations, including Pomodoro
  lengths and preferred review formats.
- **Usage:** `pace set --work 25 --break 5`

</details>

## Contributing

Found a bug? [Open an issue!](https://github.com/pace-rs/pace/issues/new/choose)

Got an idea for an improvement? Don't keep it to yourself!

- [Contribute fixes](https://github.com/pace-rs/pace/contribute) or new features
  via pull requests!

Please make sure, that you read the
[contribution guide](https://pace.cli.rs/docs/contributing_to_pace.html).

## Code of Conduct

Please review and abide by the general
[Rust Community Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct)
when contributing to this project. In the future, we might create our own Code
of Conduct and supplement it at this location.

## Acknowledgements

Some of the inspiration for `pace` came from the following projects:

- [bartib](https://github.com/nikolassv/bartib)
- [Super Productivity](https://github.com/johannesjo/super-productivity)
- [timetracking](https://github.com/hardliner66/timetracking)
- [vayu](https://github.com/MythicalCow/vayu)
- [work-break](https://github.com/ShadoySV/work-break)

## Minimum Rust version policy

This crate's minimum supported `rustc` version is `1.74.1`.

The current policy is that the minimum Rust version required to use this crate
can be increased in minor version updates. For example, if `crate 1.0` requires
Rust 1.20.0, then `crate 1.0.z` for all values of `z` will also require Rust
1.20.0 or newer. However, `crate 1.y` for `y > 0` may require a newer minimum
version of Rust.

In general, this crate will be conservative with respect to the minimum
supported version of Rust.

## License

**AGPL-3.0-or-later**; see [LICENSE](./LICENSE).
