# pace

A local-first command-line time tracker for work and daily life.

This is the restart of pace, codename **pace-ng**. Its first release is 0.16.0:
it continues the numbering of `pace-rs`, whose previous code base (last release
0.15.2) is archived.

The plan is the internal design proposal *Restarting pace*
(`src/proposals/restart.md` in the design-docs repository). Phases 1 (the
skeleton), 2 (the tracking core) and 3 (roles, clients, projects and billing)
are done.

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
  the settings, each with where it comes from; `pace config get` prints one
  setting, and `pace config set` changes one in place; `pace completions` prints
  shell completions.
- `pace setup`, the guided tour of the settings, the roles, the clients and the
  project of the working directory; it shows the changes per file and field, and
  writes only what changed once you accept them.
- Structure in plain TOML files: `role add|list|archive|edit` for the lasting
  responsibilities time goes to, `client add|list|edit` for who pays at which
  rate, and `project init|add|list|archive|edit` for the project records.
  `project init` makes the working directory a project and asks only what it
  cannot guess.
- Entries carry a role (`--role`), who decided (`--by-me`, `--by-others`),
  whether they are billable (`--billable`, `--not-billable`) and an allowed
  overlap (`--allow-overlap`).
- `pace reflect` looks back on a period: time per role, project and day, targets
  and budgets, revenue, breaks and who decided; `--compare` sets the previous
  period beside it. It writes a table, JSON, CSV, text, Markdown, HTML or your
  own Tera template.
- `pace bill` shows the bill of a client; `--mark-billed` freezes it on an
  invoice, `--invoice` shows a written invoice again, and billed entries are
  protected against changes.
- `pace export` writes every entry of a period as JSON or CSV, and `pace doctor`
  checks pace's files, entries, structure and billing, and says how to fix each
  problem.

From a project to its bill:

```console
$ pace client add acme --name "ACME Ltd" --currency EUR --rate 95 --rounding-step 15m
Added the client `acme` (ACME Ltd) in EUR at 95.00 an hour from 2026-10-01, rounding up to 15m per entry.
$ cd ~/code/shop && pace project init --client acme
Added the project `shop` (shop) for the client `acme`, billable.
Wrote the manifest .pace.toml, which links this directory to the project.
$ pace begin Write offer
Began bf64 "Write offer" at 09:00.
$ pace end
Ended bf64 "Write offer" at 11:10 after 2h 10m.
$ pace bill --month 2026-10
Bill for ACME Ltd: October 2026, Europe/Berlin

shop
  Date        Note          Hours       Rate      Amount
  2026-10-01  Write offer  2h 15m  95.00 EUR  213.75 EUR
  Total                    2h 15m             213.75 EUR

Total: 2h 15m billed of 2h 10m net, 213.75 EUR.
Hold time: 0s of 2h 10m (0%).
$ pace bill --month 2026-10 --mark-billed 2026-0042
Billed 1 entry on invoice 2026-0042: 2h 15m for 213.75 EUR.
```

The output above leaves out the lines that name the defaults pace took and where
the project came from.

## Crates

| Crate        | Directory           | Content                                                                                                                                                                                        |
| ------------ | ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pace-rs`    | `src/`              | the `pace` binary: Abscissa app, commands, directories, settings, project discovery, questions and setup, the tables and templates of `reflect` and `bill`, `doctor`                           |
| `pace_store` | `crates/pace-store` | the event store: its port, the `Tracker` that drives the core's step machine, the `files` backend with event files, index and device ID; the catalog's files; the TOML reader and field writer |
| `pace_core`  | `crates/pace-core`  | the domain: commands, entries, rules, the catalog, money and billing, the report, the bill, the export, doctor's data checks, the tracking step machine, the clock and ID traits; no I/O       |
| `pace_time`  | `crates/pace-time`  | time frames, the `--at` grammar and durations on jiff                                                                                                                                          |
| `pace_error` | `crates/pace-error` | errors with fix hints and exit codes                                                                                                                                                           |

## Development

`just check` runs formatting, clippy with warnings as errors, the tests and
codegate's quality gates (`just gate`, `.config/codegate.toml`); `just` lists
the other tasks (`bench`, `cov`, `deny`, `hack`). After cloning, `codegate
setup` installs the git hooks: pre-commit runs the fast gates, pre-push all of
them. Every source folder has a README, and the conventions are the project
guide in `.guides/pace/`.

A tag such as `pace-rs-v0.16.0` releases pace: `.github/workflows/release.yml`
builds the installers with dist (`dist-workspace.toml`), and git-cliff writes
`CHANGELOG.md` (`cliff.toml`). The developer guide's page *Release process* has
the steps.

## License

AGPL-3.0-or-later. See [LICENSE](LICENSE).
