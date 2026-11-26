# pace

A local-first command-line time tracker for work and daily life.

This is the restart of pace, codename **pace-ng**. When it is ready, it is
published as the next major version of `pace-rs`. The previous code base (last
release 0.15.2) is archived.

The plan is the internal design proposal *Restarting pace*
(`src/proposals/restart.md` in the design-docs repository). Phase 1, the
skeleton, is done; nothing is tracked yet.

## What works

`pace config show --origin` shows where pace keeps its files, which project the
working directory belongs to, and the settings in effect, each with where it
comes from.

## Crates

| Crate        | Directory           | Content                                                                             |
| ------------ | ------------------- | ----------------------------------------------------------------------------------- |
| `pace-rs`    | `src/`              | the `pace` binary: Abscissa app, commands, directories, settings, project discovery |
| `pace_core`  | `crates/pace-core`  | the domain: ports, `Command` types, IDs and keys; no I/O                            |
| `pace_time`  | `crates/pace-time`  | time frames, the `--at` grammar and durations on jiff                               |
| `pace_error` | `crates/pace-error` | errors with fix hints and exit codes                                                |

## Development

`just check` runs formatting, clippy with warnings as errors and the tests;
`just` lists the other tasks (`bench`, `cov`, `deny`, `hack`).

## License

AGPL-3.0-or-later. See [LICENSE](LICENSE).
