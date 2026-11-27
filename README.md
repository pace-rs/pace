<p align="center">
  <img
    src="https://raw.githubusercontent.com/pace-rs/assets/main/logos/readme_header.png"
    style="max-width:500px; width:100%; height: auto"
  />
</p>
<p align="center"><b>Mindful time tracking on the command line.</b></p>

<p align="center">
  <a href="https://github.com/pace-rs/pace/releases/latest"><img src="https://img.shields.io/github/v/release/pace-rs/pace?label=release" /></a>
  <a href="https://github.com/pace-rs/pace/actions/workflows/ci.yml"><img src="https://github.com/pace-rs/pace/actions/workflows/ci.yml/badge.svg" /></a>
  <a href="https://github.com/pace-rs/pace/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-AGPLv3+-red.svg" /></a>
  <a href="https://discord.gg/RKSWrAcYdG"><img src="https://img.shields.io/badge/chat-Discord-5865F2.svg" /></a>
</p>

## About

pace shows you where your time goes. You track as you work, with the breaks and
interruptions on the way. pace sorts the time by the roles you hold, your
projects and your clients. At the end of a day or a week, `pace reflect` looks
back with you. At the end of a month, `pace bill` writes the bill.

pace works right after installing: `pace begin` needs no setup. Your data stays
in plain TOML files on your own machines. Each device writes its own log, so any
file sync shares the data between computers without conflicts.

> **Note:** 0.16.0 is the first release of the restarted pace. Commands and
> files can still change between releases. The
> [release notes](https://github.com/pace-rs/pace/releases) say how.

## A day with pace

The day starts in `~/code/shop`, which `pace project init` made a project of the
client ACME Ltd in the role Work. pace takes the project from the directory.

```console
$ pace begin Write offer
Began b2e4 "Write offer" at 09:00.
$ pace hold --outer --reason "phone call"
Held b2e4 at 10:00, interrupted from outside: phone call.
$ pace resume
Resumed b2e4 at 10:20 after 20m held.
$ pace pause --lunch
Paused b2e4 at 12:00 for lunch.
$ pace resume
Resumed b2e4 at 12:45 after 45m paused.
$ pace begin Review checkout --by-others
Ended b2e4 "Write offer" at 14:30 after 4h 25m (5h 30m in all, held 20m, paused 45m).
Began accc "Review checkout" at 14:30.
$ pace end
Ended accc "Review checkout" at 16:00 after 1h 30m.
$ pace begin Run --role health
Began b18d "Run" in the role Health at 18:00.
$ pace end
Ended b18d "Run" at 19:00 after 1h.
$ pace reflect today
Today: Thu 2026-10-01, Europe/Berlin
Tracked 6h 55m, untracked 12h 35m.

Time per role
  Work    5h 55m   86%  ████████████████████████████████████████
  Health      1h   14%  ██████▊

Time per project
  shop        5h 55m   86%  ████████████████████████████████████████
  No project      1h   14%  ██████▊

Target vs. actual
  Work  5h 55m  of 5h 42m  104%

Revenue
  ACME Ltd  5h 55m  562.08 EUR
    shop    5h 55m  562.08 EUR
  Total             562.08 EUR

Interruptions
  1 hold, 20m: inner 0, outer 1 (20m), unknown 0.
  Most frequent reasons: phone call 1.

Pauses and catches
  Pauses: lunch 1 (45m).

Who decided
  By me           –
  By others  1h 30m   22%  ███████████
  Unmarked   5h 25m   78%  ████████████████████████████████████████
```

The output leaves out the lines that say where the project came from.

## What pace does

- **Tracking without setup:** `begin`, `hold`, `pause`, `resume`, `catch` and
  `end`. `now` shows what runs, also as JSON. `--at` takes times such as
  `09:30`, `-15m` or `"yesterday 17:00"`, and every moment keeps its time zone.
- **Corrections you can take back:** `adjust`, `log`, `edit` in your editor, and
  `undo`. `forget` removes an entry for good.
- **Structure in plain TOML files:** roles for the lasting responsibilities your
  time goes to, clients with their rates and rounding, and projects. `pace
  project init` makes a directory a project and asks only what it cannot guess.
  `pace setup` walks you through all of it.
- **Reflection:** `pace reflect` looks back on any period. It shows the time per
  role, project and day, targets and budgets, revenue, breaks, interruptions and
  who decided. `--compare` sets the previous period beside it. It writes a
  table, JSON, CSV, text, Markdown, HTML or your own Tera template.
- **Billing:** `pace bill` writes a client's bill, `--mark-billed` freezes it on
  an invoice, and `--invoice` shows the invoice again. pace protects billed
  entries against changes.
- **Your data stays yours:** each device writes an append-only TOML log, which
  any editor can read. `pace export` writes entries as JSON or CSV. `pace
  doctor` checks pace's files and data, and says how to fix each problem. `pace
  config show --origin` names where each setting comes from.

## From a project to its bill

```console
$ pace client add acme --name "ACME Ltd" --currency EUR --rate 95 --rounding-step 15m
Added the client `acme` (ACME Ltd) in EUR at 95.00 an hour from 2026-10-01, rounding up to 15m per entry.
$ cd ~/code/shop && pace project init --client acme
Added the project `shop` (shop) for the client `acme`, billable.
Wrote the manifest .pace.toml, which links this directory to the project.
$ pace begin Write offer
Began 0d71 "Write offer" at 09:00.
$ pace end
Ended 0d71 "Write offer" at 11:10 after 2h 10m.
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

The output leaves out the lines that name the defaults pace took and where the
project came from.

## Installation

With Homebrew on macOS and Linux:

```console
brew install pace-rs/tap/pace-rs
```

With the shell installer on macOS and Linux:

```console
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/pace-rs/pace/releases/latest/download/pace-rs-installer.sh | sh
```

With the PowerShell installer on Windows:

```console
powershell -ExecutionPolicy Bypass -c "irm https://github.com/pace-rs/pace/releases/latest/download/pace-rs-installer.ps1 | iex"
```

The [installation guide](https://pace.cli.rs/docs/user_guide/installation.html)
also covers the Windows installer package, updates, cargo-binstall, the archives
and building from source.

## Documentation

- The [user guide](https://pace.cli.rs/docs/) starts with
  [getting started](https://pace.cli.rs/docs/user_guide/getting_started.html).
  It covers [tracking](https://pace.cli.rs/docs/user_guide/tracking.html),
  [roles, clients and projects](https://pace.cli.rs/docs/user_guide/structure.html),
  [reports](https://pace.cli.rs/docs/user_guide/reports.html),
  [billing](https://pace.cli.rs/docs/user_guide/billing.html) and
  [usage examples](https://pace.cli.rs/docs/user_guide/usage_examples.html).
- `pace help` lists the commands, and `pace <command> --help` describes one.
- The [developer guide](https://pace.cli.rs/dev-docs/) describes how pace works
  inside.

## Contact

Ask questions in the [Discussions](https://github.com/orgs/pace-rs/discussions)
or on Discord.

| Contact       | Where?                                                             |
| ------------- | ------------------------------------------------------------------ |
| Issue Tracker | [GitHub Issues](https://github.com/pace-rs/pace/issues/new/choose) |
| Discord       | [pace on Discord](https://discord.gg/RKSWrAcYdG)                   |
| Discussions   | [GitHub Discussions](https://github.com/orgs/pace-rs/discussions)  |

## Contributing

Found a bug? [Open an issue](https://github.com/pace-rs/pace/issues/new/choose).
Got an idea for an improvement? Start a
[discussion](https://github.com/orgs/pace-rs/discussions), or send a pull
request.

To work on pace, read the developer guide's
[getting started](https://pace.cli.rs/dev-docs/getting_started.html). In short:

- `just check` runs formatting, clippy with warnings as errors, the tests and
  codegate's quality gates (`.config/codegate.toml`). `just` lists the other
  tasks (`bench`, `cov`, `deny`, `hack`).
- After cloning, `codegate setup` installs the git hooks: pre-commit runs the
  fast gates, pre-push all of them.
- pace's conventions are the project guide in [`.guides/pace/`](.guides/pace/).
  Every source folder has a README.
- Building pace needs Rust 1.100 or newer.

| Crate        | Directory           | Content                                                                          |
| ------------ | ------------------- | -------------------------------------------------------------------------------- |
| `pace-rs`    | `src/`              | the `pace` binary: commands, settings, project discovery, setup, output          |
| `pace_core`  | `crates/pace-core`  | the domain without I/O: entries, rules, roles, clients, projects, reports, bills |
| `pace_store` | `crates/pace-store` | the event store and its `files` backend, and the catalog's files                 |
| `pace_time`  | `crates/pace-time`  | time frames, the `--at` grammar and durations, on jiff                           |
| `pace_error` | `crates/pace-error` | errors with fix hints and exit codes                                             |

A tag such as `pace-rs-v0.16.0` releases pace. The developer guide's
[release process](https://pace.cli.rs/dev-docs/development/release_process.html)
has the steps.

## Code of Conduct

When you take part in this project, please follow the
[Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

## Acknowledgements

Some of pace's ideas come from these projects:

- [bartib](https://github.com/nikolassv/bartib)
- [Flowkeeper](https://github.com/flowkeeper-org/fk-desktop)
- [Super Productivity](https://github.com/johannesjo/super-productivity)
- [timetracking](https://github.com/hardliner66/timetracking)
- [vayu](https://github.com/MythicalCow/vayu)
- [work-break](https://github.com/ShadoySV/work-break)

## License

**AGPL-3.0-or-later**; see [LICENSE](LICENSE).
