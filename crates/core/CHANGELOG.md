# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.18.0](https://github.com/pace-rs/pace/compare/pace_core-v0.17.0...pace_core-v0.18.0) - 2024-03-24

### Added
- *(template)* implement html and markdown templating for reflections ([#105](https://github.com/pace-rs/pace/pull/105))
- *(commands)* set short version of time zone arguments and case-sensitive

## [0.17.0](https://github.com/pace-rs/pace/compare/pace_core-v0.16.1...pace_core-v0.17.0) - 2024-03-23

### Added
- *(display)* implement time zone display for activity ([#103](https://github.com/pace-rs/pace/pull/103))
- *(timezone)* improve ux and time zone handling ([#100](https://github.com/pace-rs/pace/pull/100))
- *(commands)* impl getters and setters for config values
- *(commands)* add rather bare bones settings command for now
- add arg-group to make tz args mutually exclusive
- add timezone args also to other commands
- set visible aliases
- *(setup)* implement time zone prompt for setup config
- *(commands)* rename review to reflect
- get local time offset
- *(timezone)* [**breaking**] improves how pace handles timezones

### Fixed
- *(deps)* update rust crate toml to 0.8.12 ([#99](https://github.com/pace-rs/pace/pull/99))
- *(deps)* update rust crate diesel to 2.1.5 ([#98](https://github.com/pace-rs/pace/pull/98))
- *(deps)* update rust crate wildmatch to 2.3.3 ([#95](https://github.com/pace-rs/pace/pull/95))
- clippy lints
- *(deps)* update rust crate wildmatch to 2.3.2 ([#94](https://github.com/pace-rs/pace/pull/94))
- *(deps)* update rust crate thiserror to 1.0.58 ([#93](https://github.com/pace-rs/pace/pull/93))
- *(deps)* update rust crate wildmatch to 2.3.1 ([#92](https://github.com/pace-rs/pace/pull/92))
- *(deps)* update rust crate toml to 0.8.11 ([#91](https://github.com/pace-rs/pace/pull/91))
- *(deps)* update rust crate strum_macros to 0.26.2 ([#86](https://github.com/pace-rs/pace/pull/86))
- *(deps)* update rust crate strum to 0.26.2 ([#85](https://github.com/pace-rs/pace/pull/85))

### Other
- add more tests including journey test ([#102](https://github.com/pace-rs/pace/pull/102))
- update manifests
- cleanup imports

## [0.16.1](https://github.com/pace-rs/pace/compare/pace_core-v0.16.0...pace_core-v0.16.1) - 2024-03-09

### Other
- move journey test to integration tests
- *(deps)* add missing chrono feature to diesel
- cleanup after removing rusqlite and replacing with diesel

## [0.16.0](https://github.com/pace-rs/pace/compare/pace_core-v0.15.1...pace_core-v0.16.0) - 2024-03-08

### Added
- *(config)* [**breaking**] change config fields to use kebab-case and use shorter names for general config. Please rename your old config and regenerate your config with `pace setup config`. Or replace `_` with `-` and rename the following fields accordingly:
- *(review)* add case-sensitive flag and implement filtering by categories
- *(review)* split categories into (sub-)categories and deduplicated based on that
- *(review)* the duration of running activities is taken at the time of the review, so running activities don't show up as 0 duration anymore
- *(review)* deduplicate activities within a category by description
- *(commands)* add `setup show` subcommand to show the currently loaded configuration
- *(insights)* Show amount and duration of intermissions in insights

### Other
- fix clippy lints
- *(deps)* update miette to v7.2.0
- *(deps)* update dependencies

## [0.15.1](https://github.com/pace-rs/pace/compare/pace_core-v0.15.0...pace_core-v0.15.1) - 2024-03-07

### Fixed
- *(review)* the table layout for review was broken, this has been fixed now

## [0.15.0](https://github.com/pace-rs/pace/compare/pace_core-v0.14.0...pace_core-v0.15.0) - 2024-03-07

### Added
- *(insights)* export insights to json and temporary html template ([#73](https://github.com/pace-rs/pace/pull/73))

### Fixed
- *(deps)* update rust crate chrono to 0.4.35 ([#72](https://github.com/pace-rs/pace/pull/72))

### Other
- *(time)* implement more time based functionality and add more testing ([#71](https://github.com/pace-rs/pace/pull/71))
- add more debug prints in verbose mode
- *(debug)* use tracing and debug! macro to add some more structured logging to pace_core ([#70](https://github.com/pace-rs/pace/pull/70))
- *(error)* [**breaking**] remove expect/unwrap from codebase ([#69](https://github.com/pace-rs/pace/pull/69))
- *(deps)* update rust crate insta to 1.36.1 ([#68](https://github.com/pace-rs/pace/pull/68))
- *(deps)* update rust crate insta to 1.36.0 ([#66](https://github.com/pace-rs/pace/pull/66))

## [0.14.0](https://github.com/pace-rs/pace/compare/pace_core-v0.13.0...pace_core-v0.14.0) - 2024-03-02

### Added
- add opening documentation on configuration
- *(commands)* [**breaking**] remove only-last option for end and replace --start/--end with --at/-a for setting times

### Fixed
- make sure, there are never any held activities without an active intermission
- *(deps)* update rust crate open to 5.1.0 ([#63](https://github.com/pace-rs/pace/pull/63))

### Other
- reimplement logic to end and activity for in-memory storage to make it easier for error handling
- check if activities to resume is none
- add test for beginning activies on top of held ones
- refactor tests to use results
- use is_future validator for extract_time_or_now to make sure the user doesn't use times laying in the future
- add doc comment to is_endable()

## [0.13.0](https://github.com/pace-rs/pace/compare/pace_core-v0.12.1...pace_core-v0.13.0) - 2024-03-01

### Fixed
- create parent dir and activity and config file if --activity-log-file/--config is passed to pace but not existing
- *(commands)* add short arg -s for begin --start
- *(time)* actually test if begin time lies in the future, throwing an error that begin time cannot be after end time
- *(command)* only set/override description when it actually contains a value

### Other
- fix snapshot testing for ci ([#62](https://github.com/pace-rs/pace/pull/62))
- fix test for grouping activities fail on the boundary to midnight

## [0.12.1](https://github.com/pace-rs/pace/compare/pace_core-v0.12.0...pace_core-v0.12.1) - 2024-02-29

### Added
- *(commands)* implement adjust command and update readme accordingly

### Fixed
- clippy

### Other
- *(activitylog)* do not pretty print to have collections (e.g., for tags) on one line

## [0.12.0](https://github.com/pace-rs/pace/compare/pace_core-v0.11.0...pace_core-v0.12.0) - 2024-02-28

### Fixed
- *(deps)* update rust crate log to 0.4.21 ([#57](https://github.com/pace-rs/pace/pull/57))

### Other
- *(commands)* [**breaking**] Move a lot of the commands into pace-core ([#56](https://github.com/pace-rs/pace/pull/56))

## [0.11.0](https://github.com/pace-rs/pace/compare/pace_core-v0.10.2...pace_core-v0.11.0) - 2024-02-28

### Added
- *(storage)* [**breaking**] Replace dynamic dispatch with enum dispatch ([#55](https://github.com/pace-rs/pace/pull/55))
- *(cli)* [**breaking**] move clap types into pace_core and introduce clap features for that ([#54](https://github.com/pace-rs/pace/pull/54))
- *(cli)* add cli options to review command ([#51](https://github.com/pace-rs/pace/pull/51))
- *(review)* implement some review related activity queries and their tests ([#50](https://github.com/pace-rs/pace/pull/50))

### Fixed
- *(deps)* update rust crate rayon to 1.9.0 ([#52](https://github.com/pace-rs/pace/pull/52))

### Other
- add comment about breaking changes to libraries
- add coc to readmes
- *(setup)* rename craft command to setup ([#53](https://github.com/pace-rs/pace/pull/53))

## [0.10.2](https://github.com/pace-rs/pace/compare/pace_core-v0.10.1...pace_core-v0.10.2) - 2024-02-27

### Fixed
- remove monthly and yearly bound on activity_log file name

## [0.10.1](https://github.com/pace-rs/pace/compare/pace_core-v0.10.0...pace_core-v0.10.1) - 2024-02-27

### Added
- implement continuing already ended and not held activities ([#46](https://github.com/pace-rs/pace/pull/46))

## [0.10.0](https://github.com/pace-rs/pace/compare/pace_core-v0.9.0...pace_core-v0.10.0) - 2024-02-26

### Added
- add more resume related functionality ([#44](https://github.com/pace-rs/pace/pull/44))
- [**breaking**] preparation for resuming activities from intermissions and other use cases ([#43](https://github.com/pace-rs/pace/pull/43))
- *(intermission)* implement `pace hold` and a bit of `pace resume` functionality, to be able to pause tasks ([#41](https://github.com/pace-rs/pace/pull/41))

### Other
- replace std lib rwlock with parking_lot rwlock

## [0.9.0](https://github.com/pace-rs/pace/compare/pace_core-v0.8.0...pace_core-v0.9.0) - 2024-02-22

### Fixed
- *(deps)* update serde monorepo to 1.0.197 ([#40](https://github.com/pace-rs/pace/pull/40))
- *(clippy)* apply clippy lints for main

### Other
- cleanup crates
- Improve testability and overall usability ([#37](https://github.com/pace-rs/pace/pull/37))
- add pace-server library
- update asset auto size
- update asset auto size
- *(deps)* remove unnecessary async dep bloat (tokio, condvar, futures)

## [0.8.0](https://github.com/pace-rs/pace/compare/pace_core-v0.7.0...pace_core-v0.8.0) - 2024-02-17

### Added
- *(api)* [**breaking**] refine core api and cleanup library interfaces

### Other
- implement more tests for ActivityStore and setup code coverage ([#31](https://github.com/pace-rs/pace/pull/31))
- update discussions link

## [0.7.0](https://github.com/pace-rs/pace/compare/pace_core-v0.6.0...pace_core-v0.7.0) - 2024-02-16

### Added

- *(activity)* use only seconds for duration

### Other

- *(storage)* implement in-memory storage
  ([#28](https://github.com/pace-rs/pace/pull/28))
- remove usage from library readmes as it's cumbersome to update and crates.io
  gives good advices anyway
- update domain

## [0.6.0](https://github.com/pace-rs/pace/compare/pace_core-v0.5.1...pace_core-v0.6.0) - 2024-02-15

### Added

- *(config)* Implement reading from configuration file and improve error
  handling for onboarding UX

## [0.5.1](https://github.com/pace-rs/pace/compare/pace_core-v0.5.0...pace_core-v0.5.1) - 2024-02-14

### Other

- add discord server
- *(deps)* update dependencies
- update urls
- update links to contribution guide
- fix version number for usage

## [0.5.0](https://github.com/pace-rs/pace/compare/pace_core-v0.4.0...pace_core-v0.5.0) - 2024-02-14

### Added

- *(commands)* introduce new `pace craft setup` command to make onboarding
  easier ([#10](https://github.com/pace-rs/pace/pull/10))

### Other

- update readmes
- update cargo manifests

## [0.4.0](https://github.com/pace-rs/pace/compare/pace_core-v0.3.0...pace_core-v0.4.0) - 2024-02-12

### Added

- *(core)* subdivide storage trait and apply fixes
  ([#3](https://github.com/pace-rs/pace/pull/3))

## [0.3.0](https://github.com/pace-rs/pace/compare/pace_core-v0.2.0...pace_core-v0.3.0) - 2024-02-10

### Added

- *(commands)* [**breaking**] implement `begin`, `end`, and `now` command
  ([#1](https://github.com/pace-rs/pace/pull/1))
- *(core)* add core library

### Fixed

- *(manifest)* [**breaking**] use includes to only package the bare minimum for
  crates.io

### Other

- pace 0.3.0 / pace-core 0.2.0
- pace 0.2.0 / pace-core 0.1.1

## [0.2.0](https://github.com/pace-rs/pace/compare/pace_core-v0.1.1...pace_core-v0.2.0) - 2024-02-03

### Fixed

- *(manifest)* [**breaking**] use includes to only package the bare minimum for
  crates.io

### Other

- pace 0.2.0 / pace-core 0.1.1

## [0.1.1](https://github.com/pace-rs/pace/compare/pace_core-v0.1.0...pace_core-v0.1.1) - 2024-02-03

### Added

- *(core)* add core library
