# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.3](https://github.com/pace-rs/pace/compare/pace_cli-v0.4.2...pace_cli-v0.4.3) - 2024-03-08

### Other
- fix clippy lints

## [0.4.2](https://github.com/pace-rs/pace/compare/pace_cli-v0.4.1...pace_cli-v0.4.2) - 2024-03-07

### Fixed
- *(deps)* update rust crate chrono to 0.4.35 ([#72](https://github.com/pace-rs/pace/pull/72))

### Other
- pull out art for easier replacement

## [0.4.1](https://github.com/pace-rs/pace/compare/pace_cli-v0.4.0...pace_cli-v0.4.1) - 2024-02-29

### Fixed
- clippy

## [0.4.0](https://github.com/pace-rs/pace/compare/pace_cli-v0.3.0...pace_cli-v0.4.0) - 2024-02-28

### Other
- *(commands)* [**breaking**] Move a lot of the commands into pace-core ([#56](https://github.com/pace-rs/pace/pull/56))

## [0.3.0](https://github.com/pace-rs/pace/compare/pace_cli-v0.2.4...pace_cli-v0.3.0) - 2024-02-28

### Other
- add comment about breaking changes to libraries
- add coc to readmes
- *(setup)* rename craft command to setup ([#53](https://github.com/pace-rs/pace/pull/53))

## [0.2.4](https://github.com/pace-rs/pace/compare/pace_cli-v0.2.3...pace_cli-v0.2.4) - 2024-02-27

### Fixed
- remove monthly and yearly bound on activity_log file name

## [0.2.3](https://github.com/pace-rs/pace/compare/pace_cli-v0.2.2...pace_cli-v0.2.3) - 2024-02-27

### Added
- implement continuing already ended and not held activities ([#46](https://github.com/pace-rs/pace/pull/46))

## [0.2.2](https://github.com/pace-rs/pace/compare/pace_cli-v0.2.1...pace_cli-v0.2.2) - 2024-02-26

### Added
- add more resume related functionality ([#44](https://github.com/pace-rs/pace/pull/44))
- *(intermission)* implement `pace hold` and a bit of `pace resume` functionality, to be able to pause tasks ([#41](https://github.com/pace-rs/pace/pull/41))

## [0.2.1](https://github.com/pace-rs/pace/compare/pace_cli-v0.2.0...pace_cli-v0.2.1) - 2024-02-22

### Other
- Improve testability and overall usability ([#37](https://github.com/pace-rs/pace/pull/37))
- add pace-server library
- update asset auto size
- update asset auto size

## [0.2.0](https://github.com/pace-rs/pace/compare/pace_cli-v0.1.3...pace_cli-v0.2.0) - 2024-02-17

### Added
- *(api)* [**breaking**] refine core api and cleanup library interfaces

### Other
- update discussions link

## [0.1.3](https://github.com/pace-rs/pace/compare/pace_cli-v0.1.2...pace_cli-v0.1.3) - 2024-02-16

### Other

- *(storage)* implement in-memory storage
  ([#28](https://github.com/pace-rs/pace/pull/28))
- remove usage from library readmes as it's cumbersome to update and crates.io
  gives good advices anyway
- update domain

## [0.1.2](https://github.com/pace-rs/pace/compare/pace_cli-v0.1.1...pace_cli-v0.1.2) - 2024-02-15

### Fixed

- typo in setup intro text

## [0.1.1](https://github.com/pace-rs/pace/compare/pace_cli-v0.1.0...pace_cli-v0.1.1) - 2024-02-14

### Other

- add discord server
- update urls
- update links to contribution guide
- fix version number for usage

## [0.1.0](https://github.com/pace-rs/pace/releases/tag/pace_cli-v0.1.0) - 2024-02-14

### Added

- *(commands)* introduce new `pace craft setup` command to make onboarding
  easier ([#10](https://github.com/pace-rs/pace/pull/10))

### Other

- update readmes
- update cargo manifests
