# `pace`

The app crate `pace-rs`: the Abscissa application and the `pace` command line on
top of the core. `application.rs` resolves the directories, the settings and the
project before a command runs. `commands/` holds the commands, and `session.rs`
opens the tracker over the `files` backend. What the user reads comes from
`output.rs`, and what pace asks from `prompt.rs`.

## Files

<!-- files -->

| File                               | What it holds                                                                                                                      |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| [`application.rs`](application.rs) | The Abscissa application.                                                                                                          |
| [`clock.rs`](clock.rs)             | The adapters for the `Clock` and `Ids` ports.                                                                                      |
| [`commands.rs`](commands.rs)       | The command line: the entry point and its subcommands.                                                                             |
| [`config.rs`](config.rs)           | Settings and the layers they come from.                                                                                            |
| [`dirs.rs`](dirs.rs)               | The three places pace keeps files: the global config file, the data directory (`PACE_HOME`) and the machine directory.             |
| [`editor.rs`](editor.rs)           | Editing text in the user's editor: `VISUAL`, else `EDITOR`.                                                                        |
| [`lib.rs`](lib.rs)                 | pace: a local-first command-line time tracker for work and daily life.                                                             |
| [`origin.rs`](origin.rs)           | Where a setting or a directory comes from.                                                                                         |
| [`output.rs`](output.rs)           | How tracking commands describe what they did.                                                                                      |
| [`project.rs`](project.rs)         | The project context: which project the working directory belongs to.                                                               |
| [`prompt.rs`](prompt.rs)           | Asking the user, only when a terminal is attached.                                                                                 |
| [`select.rs`](select.rs)           | Choosing the entry a command acts on: by its ID or short ID, `--last`, or the open entry.                                          |
| [`session.rs`](session.rs)         | What a tracking command works with: the tracker over the files backend, with this machine's device ID, the clock and the settings. |
| [`bin/`](bin/)                     |                                                                                                                                    |
| [`commands/`](commands/)           |                                                                                                                                    |
| [`dirs/`](dirs/)                   |                                                                                                                                    |

<!-- /files -->

## How it connects

<!-- uses -->

The crate root declares the modules. Each folder's README names what it uses.

<!-- /uses -->
