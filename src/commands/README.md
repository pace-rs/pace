# `commands`

The commands, one file per command or group of commands; `src/commands.rs`
declares the entry point and the subcommands. A tracking command opens a
`Session`, resolves the project, the entry and the times, and hands a
`pace_core::Command` to the tracker.

## Files

<!-- files -->

| File                               | What it holds                                                                         |
| ---------------------------------- | ------------------------------------------------------------------------------------- |
| [`completions.rs`](completions.rs) | `pace completions`: the completion script for a shell.                                |
| [`config.rs`](config.rs)           | `pace config`                                                                         |
| [`corrections.rs`](corrections.rs) | Correcting the record: `adjust`, `log`, `undo`, `forget`.                             |
| [`edit.rs`](edit.rs)               | `pace edit`: an entry as TOML in the user's editor; the changes become an adjustment. |
| [`now.rs`](now.rs)                 | `pace now`: what is open, and where new entries go.                                   |
| [`store.rs`](store.rs)             | `pace store`: how pace keeps its data.                                                |
| [`tracking.rs`](tracking.rs)       | The tracking commands: `begin`, `hold`, `pause`, `resume`, `catch`, `end`.            |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`application`](../application.rs), [`dirs`](../dirs/),
  [`editor`](../editor.rs), [`output`](../output.rs),
  [`project`](../project.rs), [`prompt`](../prompt.rs),
  [`select`](../select.rs), [`session`](../session.rs).
- Used by: [`application`](../application.rs).

<!-- /uses -->
