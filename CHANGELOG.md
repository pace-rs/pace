# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
