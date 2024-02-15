<p align="center">
<img src="https://raw.githubusercontent.com/pace-rs/assets/main/logos/readme_header_core.png" height="350" />
</p>
<p align="center"><b>pace-core - library to support timetracking on the command line</b></p>

<p align="center">
<a href="https://crates.io/crates/pace_core"><img src="https://img.shields.io/crates/v/pace_core.svg" /></a>
<a href="https://docs.rs/pace_core/"><img src="https://img.shields.io/docsrs/pace_core?style=flat&amp;labelColor=1c1d42&amp;color=4f396a&amp;logo=Rust&amp;logoColor=white" /></a>
<a href="https://raw.githubusercontent.com/pace-rs/pace/main/crates/core/LICENSE"><img src="https://img.shields.io/badge/license-AGPLv3+-red.svg" /></a>
<a href="https://crates.io/crates/pace_core"><img src="https://img.shields.io/crates/d/pace_core.svg" /></a>
<p>

## About

`pace-core` is a library to support timetracking on the command line. It is the
core library for the `pace` timetracking application.

## Contact

You can ask questions in the
[Discussions](https://github.com/pace-rs/pace/discussions) or have a look at the
[FAQ](https://pace.cli.rs/docs/FAQ.html).

| Contact       | Where?                                                                                                          |
| ------------- | --------------------------------------------------------------------------------------------------------------- |
| Issue Tracker | [GitHub Issues](https://github.com/pace-rs/pace/issues/new/choose)                                              |
| Discord       | [![Discord](https://dcbadge.vercel.app/api/server/RKSWrAcYdG?style=flat-square)](https://discord.gg/RKSWrAcYdG) |
| Discussions   | [GitHub Discussions](https://github.com/pace-rs/discussions)                                                    |

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
pace_core = "0.5"
```

or use

```console
cargo add pace_core
```

## Features

- **Time Tracking**: Track your time on the command line.
- **Simple**: Easy to use and understand.
- **Fast**: Written in Rust, `pace-core` is fast and efficient

## Examples

TODO!

## Contributing

Found a bug? [Open an issue!](https://github.com/pace-rs/pace/issues/new/choose)

Got an idea for an improvement? Don't keep it to yourself!

- [Contribute fixes](https://github.com/pace-rs/pace/contribute) or new features
  via a pull requests!

Please make sure, that you read the
[contribution guide](https://pace.cli.rs/docs/contributing_to_pace.html).

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
