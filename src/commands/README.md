# `commands`

The commands, one file per command or group of commands; `src/commands.rs`
declares the entry point and the subcommands. A tracking command opens a
`Session`, resolves the project, the entry and the times, and hands a
`pace_core::Command` to the tracker. A command that adds something, such as
`role add`, `client add`, `project add` or `project init`, drives a flow of
`questions` and writes its field edits. `reflect` reads the period's events,
builds the `pace_core::Report` and draws it with `render`. `bill` picks the
client, reads the events from the period's start on, so the state holds every
entry that may overlap one on the bill, builds the `pace_core::Bill` and draws
it with `render` too. `setup` drives the step machine of `setup.rs` through the
asker, and writes the edits once the user accepts the summary. `role edit`,
`client edit` and `project edit` open a file, or one client's table, in the
editor until the catalog reads it; `project edit` also refuses a changed key or
ID.

## Files

<!-- files -->

| File                                 | What it holds                                                                                                                       |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| [`bill.rs`](bill.rs)                 | `pace bill`: the bill of a client over a period, as a table, JSON, CSV or Markdown from a template.                                 |
| [`client.rs`](client.rs)             | `pace client`: add, list and edit the clients in `clients.toml`.                                                                    |
| [`completions.rs`](completions.rs)   | `pace completions`: the completion script for a shell.                                                                              |
| [`config.rs`](config.rs)             | `pace config`                                                                                                                       |
| [`corrections.rs`](corrections.rs)   | Correcting the record: `adjust`, `log`, `undo`, `forget`.                                                                           |
| [`edit.rs`](edit.rs)                 | `pace edit`: an entry as TOML in the user's editor; the changes become an adjustment.                                               |
| [`entry_fields.rs`](entry_fields.rs) | The flags of an entry's role, who decided and billable, which `begin`, `log` and `adjust` share.                                    |
| [`now.rs`](now.rs)                   | `pace now`: what is open, and where new entries go.                                                                                 |
| [`project.rs`](project.rs)           | `pace project`: make the working directory a project, and add, list, archive and edit the project records in `projects/<key>.toml`. |
| [`project_init.rs`](project_init.rs) | `pace project init`: make the working directory a project, with the record and the manifest that links the directory to it.         |
| [`reflect.rs`](reflect.rs)           | `pace reflect`: the report of a period, as a table, JSON, CSV, or text, Markdown and HTML from a template.                          |
| [`role.rs`](role.rs)                 | `pace role`: add, list, archive and edit the roles in `roles.toml`.                                                                 |
| [`setup.rs`](setup.rs)               | `pace setup`: the guided tour of the settings, the roles, the clients and this directory's project, which writes only what changed. |
| [`store.rs`](store.rs)               | `pace store`: how pace keeps its data.                                                                                              |
| [`tracking.rs`](tracking.rs)         | The tracking commands: `begin`, `hold`, `pause`, `resume`, `catch`, `end`.                                                          |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`application`](../application.rs), [`clock`](../clock.rs),
  [`config`](../config/), [`dirs`](../dirs/), [`editor`](../editor.rs),
  [`origin`](../origin.rs), [`output`](../output.rs), [`print`](../print.rs),
  [`project`](../project.rs), [`prompt`](../prompt.rs),
  [`questions`](../questions/), [`render`](../render/),
  [`select`](../select.rs), [`session`](../session.rs), [`setup`](../setup/).
- Used by: [`application`](../application.rs), [`editor`](../editor.rs).

<!-- /uses -->
