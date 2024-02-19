<p align="center">
<img src="https://raw.githubusercontent.com/pace-rs/assets/main/logos/readme_header_server.png" style="max-width:500px; width:100%; height: auto" />
</p>
<p align="center"><b>pace-server - server library to support timetracking on the command line</b></p>

<p align="center">
<a href="https://crates.io/crates/pace_server"><img src="https://img.shields.io/crates/v/pace_server.svg" /></a>
<a href="https://docs.rs/pace_server/"><img src="https://img.shields.io/docsrs/pace_server?style=flat&amp;labelColor=1c1d42&amp;color=4f396a&amp;logo=Rust&amp;logoColor=white" /></a>
<a href="https://raw.githubusercontent.com/pace-rs/pace/main/crates/server/LICENSE"><img src="https://img.shields.io/badge/license-AGPLv3+-red.svg" /></a>
<a href="https://crates.io/crates/pace_server"><img src="https://img.shields.io/crates/d/pace_server.svg" /></a>
<p>

## About

`pace_server` is a library to support timetracking on the command line. It is
the server library for the `pace` timetracking application. In the near future,
it will be used to provide a server for the `pace` command line application to
store and retrieve time tracking data.

**NOTE**: This library is currently under heavy development and not yet ready
for production use. This is a placeholder crate to reserve the name on
crates.io.

## Contact

You can ask questions in the
[Discussions](https://github.com/orgs/pace-rs/discussions) or have a look at the
[FAQ](https://pace.cli.rs/docs/FAQ.html).

| Contact       | Where?                                                                                                          |
| ------------- | --------------------------------------------------------------------------------------------------------------- |
| Issue Tracker | [GitHub Issues](https://github.com/pace-rs/pace/issues/new/choose)                                              |
| Discord       | [![Discord](https://dcbadge.vercel.app/api/server/RKSWrAcYdG?style=flat-square)](https://discord.gg/RKSWrAcYdG) |
| Discussions   | [GitHub Discussions](https://github.com/orgs/pace-rs/discussions)                                               |

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
