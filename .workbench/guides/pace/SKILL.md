---
name: pace
description: pace's own conventions for /work/pace and its books (docs, dev-docs, design) — errors with PaceError and exit codes, commands, flags and output, the core without I/O and its ports, tests (journey tests, snapshots, ID placeholders, test names), lints and codegate's gates, and the docs style. Load before writing, reviewing or naming pace's code, tests or docs; it refines rust, rust-architecture, code-principles and prose.
---

# pace

The conventions of pace-ng, the restarted pace. The built-in guides
`code-principles`, `rust`, `rust-architecture` and `prose` hold for pace too;
this guide sits below them and wins where it says more.

The plan behind the rules is the design proposal *Restarting pace*
(`design/src/proposals/restart.md`). What the code does today is in the
developer guide (`dev-docs/src/architecture/`, `dev-docs/src/development/`).

## Index — load the matching section on demand

| #  | File                                     | Content                                                            | When to read                                        |
| -- | ---------------------------------------- | ------------------------------------------------------------------ | --------------------------------------------------- |
| 01 | [errors](01-errors.md)                   | `PaceError`, guidance, context, hints, exit codes                  | An error path or a message                          |
| 02 | [CLI and output](02-cli-and-output.md)   | Verbs, flags, one sentence of output, no waiting without terminal  | A command, a flag, what a command prints            |
| 03 | [architecture](03-architecture.md)       | The core without I/O, ports, one runtime, jiff, events, settings   | A new module, port or backend; time; a setting      |
| 04 | [tests](04-tests.md)                     | Tests first, journey tests, snapshots, ID placeholders, test names | Writing or changing a test                          |
| 05 | [lints and gates](05-lints-and-gates.md) | Workspace lints, codegate's gates, the checks before a commit      | Before a commit; a lint, a long file or function    |
| 06 | [docs](06-docs.md)                       | The three books, their style, proposals, commit messages           | User docs, dev-docs, a proposal, a README, a commit |

## Status

Moved from `/work/CLAUDE.md` on 2026-11-26, when the guides MCP and codegate
were set up for pace. A rule that is pace's own goes here, not into `CLAUDE.md`.
