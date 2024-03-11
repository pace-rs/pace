<p align="center">
<img src="https://raw.githubusercontent.com/pace-rs/assets/main/logos/readme_header.png" style="max-width:500px; width:100%; height: auto" />
</p>
<p align="center"><b>Mindful Time Tracking: Simplify Your Focus and Boost Productivity Effortlessly.</b></p>

<p align="center">
<a href="https://crates.io/crates/pace-rs"><img src="https://img.shields.io/crates/v/pace-rs.svg" /></a>
<a href="https://docs.rs/pace-rs/"><img src="https://img.shields.io/docsrs/pace-rs?style=flat&amp;labelColor=1c1d42&amp;color=4f396a&amp;logo=Rust&amp;logoColor=white" /></a>
<a href="https://codecov.io/gh/pace-rs/pace" ><img src="https://codecov.io/gh/pace-rs/pace/graph/badge.svg?token=7V1G5GLG3D"/></a>
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

## Installation

Help for installing `pace` can be found in the
[installation instructions](https://pace.cli.rs/docs/installation.html).

## Getting started

Please check our
[getting started guide](https://pace.cli.rs/docs/user_guide/getting_started.html)
for more information on how to get started right afterwards. You can also run
`pace docs` to open the documentation in your browser.

## Usage

For usage examples for various commands please check the
[usage examples](https://pace.cli.rs/docs/user_guide/usage_examples.html).

## FAQ / FATQ

Please check our [FAQ](https://pace.cli.rs/docs/user_guide/FAQ.html) for
frequently asked questions. If you have a question that is not answered there,
please open an issue or ask in the discussions. We will be happy to help you. If
your are more interested in the development of `pace`, please check our
[FATQ](https://pace.cli.rs/dev-docs/appendix/FATQ.html).

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
