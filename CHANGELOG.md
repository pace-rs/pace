# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.13.0](https://github.com/pace-rs/pace/compare/pace-rs-v0.12.0...pace-rs-v0.13.0) - 2024-03-07

### Added
- *(insights)* export insights to json and temporary html template ([#73](https://github.com/pace-rs/pace/pull/73))

### Fixed
- *(deps)* update rust crate chrono to 0.4.35 ([#72](https://github.com/pace-rs/pace/pull/72))

### Other
- *(time)* implement more time based functionality and add more testing ([#71](https://github.com/pace-rs/pace/pull/71))
- add more debug prints in verbose mode
- pull out art for easier replacement
- *(debug)* use tracing and debug! macro to add some more structured logging to pace_core ([#70](https://github.com/pace-rs/pace/pull/70))
- *(error)* [**breaking**] remove expect/unwrap from codebase ([#69](https://github.com/pace-rs/pace/pull/69))
- *(deps)* move insta to dev dependencies
- *(deps)* update rust crate insta to 1.36.1 ([#68](https://github.com/pace-rs/pace/pull/68))
- *(deps)* update rust crate insta to 1.36.0 ([#66](https://github.com/pace-rs/pace/pull/66))

## [0.12.0](https://github.com/pace-rs/pace/compare/pace-rs-v0.11.1...pace-rs-v0.12.0) - 2024-03-02

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
- add comment about use cases still to test via cli
- refactor tests to use results
- use is_future validator for extract_time_or_now to make sure the user doesn't use times laying in the future
- add doc comment to is_endable()

## [0.11.1](https://github.com/pace-rs/pace/compare/pace-rs-v0.11.0...pace-rs-v0.11.1) - 2024-03-01

### Fixed
- create parent dir and activity and config file if --activity-log-file/--config is passed to pace but not existing
- *(cli)* set aliases to subcommands to visible
- phrasing in confirmation for not being able to resume ended activity
- *(commands)* add short arg -s for begin --start
- *(time)* actually test if begin time lies in the future, throwing an error that begin time cannot be after end time
- *(command)* only set/override description when it actually contains a value

### Other
- remove version snapshot
- fix snapshot testing for ci ([#62](https://github.com/pace-rs/pace/pull/62))
- factor out begin command for keeping it dry
- fix missing id for upload of snapshots
- upload insta snapshots from failed ci runs
- implement snapshot tests for cli output
- *(deps)* lock file maintenance ([#61](https://github.com/pace-rs/pace/pull/61))
- fix test for grouping activities fail on the boundary to midnight

## [0.11.0](https://github.com/pace-rs/pace/compare/pace-rs-v0.10.0...pace-rs-v0.11.0) - 2024-02-29

### Added
- *(commands)* implement adjust command and update readme accordingly
- re-export pace libraries

### Fixed
- clippy

### Other
- *(activitylog)* do not pretty print to have collections (e.g., for tags) on one line
- make just and dprint files hidden
- update scoop manifest

## [0.10.0](https://github.com/pace-rs/pace/compare/pace-rs-v0.9.0...pace-rs-v0.10.0) - 2024-02-28

### Fixed
- *(deps)* update rust crate log to 0.4.21 ([#57](https://github.com/pace-rs/pace/pull/57))

### Other
- *(commands)* [**breaking**] Move a lot of the commands into pace-core ([#56](https://github.com/pace-rs/pace/pull/56))

## [0.9.0](https://github.com/pace-rs/pace/compare/pace-rs-v0.8.2...pace-rs-v0.9.0) - 2024-02-28

### Added
- *(storage)* [**breaking**] Replace dynamic dispatch with enum dispatch ([#55](https://github.com/pace-rs/pace/pull/55))
- *(cli)* [**breaking**] move clap types into pace_core and introduce clap features for that ([#54](https://github.com/pace-rs/pace/pull/54))
- *(aliases)* add aliases to the root cli commands
- *(cli)* add cli options to review command ([#51](https://github.com/pace-rs/pace/pull/51))
- *(review)* implement some review related activity queries and their tests ([#50](https://github.com/pace-rs/pace/pull/50))

### Fixed
- *(deps)* update rust crate rayon to 1.9.0 ([#52](https://github.com/pace-rs/pace/pull/52))

### Other
- add comment about breaking changes to libraries
- add coc to readmes
- add comment about breaking changes
- *(setup)* rename craft command to setup ([#53](https://github.com/pace-rs/pace/pull/53))
- *(cli)* [**breaking**] remove -c and -a short options for config and activity log files
- update scoop manifest

## [0.8.2](https://github.com/pace-rs/pace/compare/pace-rs-v0.8.1...pace-rs-v0.8.2) - 2024-02-27

### Fixed
- remove monthly and yearly bound on activity_log file name

## [0.8.1](https://github.com/pace-rs/pace/compare/pace-rs-v0.8.0...pace-rs-v0.8.1) - 2024-02-27

### Added
- implement continuing already ended and not held activities ([#46](https://github.com/pace-rs/pace/pull/46))

### Other
- dprint fmt
- update scoop manifest
- update brew install instructions
- *(deps)* update rust crate tempfile to 3.10.1 ([#45](https://github.com/pace-rs/pace/pull/45))

## [0.8.0](https://github.com/pace-rs/pace/compare/pace-rs-v0.7.1...pace-rs-v0.8.0) - 2024-02-26

### Added
- add more resume related functionality ([#44](https://github.com/pace-rs/pace/pull/44))
- [**breaking**] preparation for resuming activities from intermissions and other use cases ([#43](https://github.com/pace-rs/pace/pull/43))
- *(intermission)* implement `pace hold` and a bit of `pace resume` functionality, to be able to pause tasks ([#41](https://github.com/pace-rs/pace/pull/41))

### Other
- replace std lib rwlock with parking_lot rwlock
- update cargo-dist and add homebrew support
- add github release to installation instructions
- add some more installation instructions and todos
- add other installation methods to readme
- *(packaging)* add scoop manifest
- *(packaging)* add cargo-binstall support

## [0.7.1](https://github.com/pace-rs/pace/compare/pace-rs-v0.7.0...pace-rs-v0.7.1) - 2024-02-22

### Fixed
- *(deps)* update serde monorepo to 1.0.197 ([#40](https://github.com/pace-rs/pace/pull/40))
- *(clippy)* apply clippy lints for main

### Other
- update command overview to reflect better the implementation status of several commands
- cleanup crates
- Improve testability and overall usability ([#37](https://github.com/pace-rs/pace/pull/37))
- *(deps)* update rust crate assert_cmd to 2.0.14 ([#39](https://github.com/pace-rs/pace/pull/39))
- update changelog
- add pace-server library
- harmonize github workflows
- update asset auto size
- update asset auto size
- rework key for commands and enhanced plausibility
- *(deps)* remove unnecessary async dep bloat (tokio, condvar, futures)

## [0.7.0](https://github.com/pace-rs/pace/compare/pace-rs-v0.6.3...pace-rs-v0.7.0) - 2024-02-17

### Added
- *(api)* [**breaking**] refine core api and cleanup library interfaces

### Fixed
- *(deps)* update rust crate clap_complete to 4.5.1 ([#33](https://github.com/pace-rs/pace/pull/33))

### Other
- format dprint.json
- ignore changelog for easier releases
- revert fix body in changelog for release-plz because it's not useful
- fix body in changelog for release-plz
- remove digests from steps, due high maintenance effort
- implement more tests for ActivityStore and setup code coverage ([#31](https://github.com/pace-rs/pace/pull/31))
- update discussions link

## [0.6.3](https://github.com/pace-rs/pace/compare/pace-rs-v0.6.2...pace-rs-v0.6.3) - 2024-02-16

### Added

- *(activity)* use only seconds for duration
- *(cli)* implement overriding config values with global cli arguments
  (-a/--activity_log_file)

### Other

- *(storage)* implement in-memory storage
  ([#28](https://github.com/pace-rs/pace/pull/28))
- remove usage from library readmes as it's cumbersome to update and crates.io
  gives good advices anyway
- add pr command to Justfile
- update example data filename
- add release plz action
- update domain

## [0.6.2](https://github.com/pace-rs/pace/compare/pace-rs-v0.6.1...pace-rs-v0.6.2) - 2024-02-15

### Added

- *(config)* Implement reading from configuration file and improve error
  handling for onboarding UX

### Fixed

- *(data)* apply fixes to example data for making the tests pass again
- typo in setup intro text

### Other

- pace_core-v0.6.0
- pace_cli v0.1.2

## [0.6.1](https://github.com/pace-rs/pace/compare/pace-rs-v0.6.0...pace-rs-v0.6.1) - 2024-02-14

### Other

- *(pace_core)* release v0.5.1 ([#23](https://github.com/pace-rs/pace/pull/23))
- *(pace_cli)* release v0.1.1 ([#21](https://github.com/pace-rs/pace/pull/21))
- add pull request template
- fix product-scope in feature request template
- add issue and feature request template
- add discord server
- *(deps)* update dependencies
- fmt
- update urls
- remove pinning for action digests
- update links to contribution guide

## [0.6.0](https://github.com/pace-rs/pace/compare/pace-rs-v0.5.0...pace-rs-v0.6.0) - 2024-02-14

### Added

- *(commands)* introduce new `pace craft setup` command to make onboarding
  easier ([#10](https://github.com/pace-rs/pace/pull/10))

### Other

- fix version number for usage
- *(pace_core)* release v0.5.0 ([#17](https://github.com/pace-rs/pace/pull/17))
- *(pace_cli)* release v0.1.0 ([#14](https://github.com/pace-rs/pace/pull/14))
- update cargo-dist
- change slogan
- update readmes
- remove assets as they can be found in pace-rs/assets repository now
- update cargo manifests
- add msrv check
- add logos from assets repository
- fmt
- add git-town config

## [0.5.0](https://github.com/pace-rs/pace/compare/pace-rs-v0.4.0...pace-rs-v0.5.0) - 2024-02-12

### Added

- *(core)* subdivide storage trait and apply fixes
  ([#3](https://github.com/pace-rs/pace/pull/3))

## [0.4.0](https://github.com/pace-rs/pace/compare/pace-rs-v0.3.0...pace-rs-v0.4.0) - 2024-02-10

### Added

- *(commands)* [**breaking**] implement `begin`, `end`, and `now` command
  ([#1](https://github.com/pace-rs/pace/pull/1))
- *(core)* add core library
- *(commands)* add commands skeleton

### Fixed

- *(manifest)* [**breaking**] use includes to only package the bare minimum for
  crates.io

### Other

- pace 0.3.0 / pace-core 0.2.0
- pace 0.2.0 / pace-core 0.1.1
- *(assets)* add logo to readme
- fix order
- add command overview
- remove unneeded feature default
- add deny.toml
- remove outdated acceptance tests
- rename pacers into pace
- add justfile for dev
- *(deps)* update dependencies
- fmt
- add build profiles
- add dprint config
- add more checks
- add renovate.json
- add cargo dist for releases
- fix manifest
- Fix Readme
- Initial commit :rocket:

## [0.3.0](https://github.com/pace-rs/pace/compare/pace-rs-v0.2.0...pace-rs-v0.3.0) - 2024-02-03

### Fixed

- *(manifest)* [**breaking**] use includes to only package the bare minimum for
  crates.io

## [0.2.0](https://github.com/pace-rs/pace/compare/pace-rs-v0.1.0...pace-rs-v0.2.0) - 2024-02-03

### Added

- *(core)* add core library
- *(commands)* add commands skeleton

### Other

- *(assets)* add logo to readme
- fix order
- add command overview
- remove unneeded feature default
- add deny.toml
- remove outdated acceptance tests
- rename pacers into pace
- add justfile for dev
- *(deps)* update dependencies
- fmt
- add build profiles
- add dprint config
- add more checks
- add renovate.json
- add cargo dist for releases
- fix manifest
- Fix Readme
- Initial commit :rocket:
