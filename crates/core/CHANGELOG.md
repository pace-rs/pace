# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
