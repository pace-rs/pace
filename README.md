# pace

A local-first command-line time tracker for work and daily life.

This is the restart of pace, codename **pace-ng**. When it is ready, it is
published as the next major version of `pace-rs`. The previous code base (last
release 0.15.2) is archived.

The plan is the internal design proposal *Restarting pace*
(`src/proposals/restart.md` in the design-docs repository). Phases 1 (the
skeleton) and 2 (the tracking core) are done; clients, projects and billing come
next.

## What works

```console
$ pace begin Write offer --tag offer
Began ad5e "Write offer" at 09:00.
$ pace hold --outer --reason "phone call"
Held ad5e at 10:00, interrupted from outside: phone call.
$ pace resume
Resumed ad5e at 10:20 after 20m held.
$ pace end
Ended ad5e "Write offer" at 12:00 after 2h 40m (3h in all, held 20m).
```

- Tracking with zero setup: `begin`, `hold`, `pause`, `resume`, `catch`, `end`,
  and `now` (also as `--json`).
- Corrections that can be taken back: `adjust`, `log`, `edit` (in your editor),
  `undo`; `forget` removes an entry for good.
- The project from the working directory (`.pace.toml`), entries chosen by short
  ID, `--at` times like `09:30`, `-15m` or `yesterday 17:00`, and every moment
  with its time zone.
- One append-only TOML event log per device, so the data directory can be synced
  between computers without conflicts; `pace store split` changes how the files
  are split.
- `pace config show --origin` shows where pace keeps its files, the project and
  the settings, each with where it comes from; `pace completions` prints shell
  completions.

## Crates

| Crate        | Directory           | Content                                                                             |
| ------------ | ------------------- | ----------------------------------------------------------------------------------- |
| `pace-rs`    | `src/`              | the `pace` binary: Abscissa app, commands, directories, settings, project discovery |
| `pace_store` | `crates/pace-store` | the `files` backend: event files, index, device ID; the TOML reader                 |
| `pace_core`  | `crates/pace-core`  | the domain: commands, entries, rules, the tracker and its ports; no I/O             |
| `pace_time`  | `crates/pace-time`  | time frames, the `--at` grammar and durations on jiff                               |
| `pace_error` | `crates/pace-error` | errors with fix hints and exit codes                                                |

## Development

`just check` runs formatting, clippy with warnings as errors, the tests and
codegate's quality gates (`just gate`, `.config/codegate.toml`); `just` lists
the other tasks (`bench`, `cov`, `deny`, `hack`). After cloning, `codegate
setup` installs the pre-commit hook that runs the fast gates. Every source
folder has a README, and the conventions are the project guide in
`.guides/pace/`.

## License

AGPL-3.0-or-later. See [LICENSE](LICENSE).
