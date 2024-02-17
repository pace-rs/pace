# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
