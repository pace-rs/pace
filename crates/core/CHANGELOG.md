# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.1](https://github.com/pace-rs/pace/compare/pace_core-v0.8.0...pace_core-v0.8.1) - 2024-02-19

### Other

- add pace-server library
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
