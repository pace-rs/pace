# `pace`

The app crate `pace-rs`: the Abscissa application and the `pace` command line on
top of the core. `application.rs` resolves the directories, the settings and the
project before a command runs. It is the only place that reads the environment:
the terminal, the editor, the hostname and `PACE_TEST_NOW` reach the commands in
the `Context`. `commands/` holds the commands, and `session.rs` opens the
tracker over the `files` backend. The commands write what the user reads through
`print.rs`; `output.rs` words the sentences of the commands, and `prompt.rs`
asks the user. A command that adds something asks through the question machine
in `questions.rs`: its flow gives the questions without I/O.

## Files

<!-- files -->

| File                               | What it holds                                                                                                                                                                 |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`application.rs`](application.rs) | The Abscissa application.                                                                                                                                                     |
| [`clock.rs`](clock.rs)             | The adapters for the `Clock` and `Ids` ports.                                                                                                                                 |
| [`commands.rs`](commands.rs)       | The command line: the entry point and its subcommands.                                                                                                                        |
| [`config.rs`](config.rs)           | Settings and the layers they come from.                                                                                                                                       |
| [`dirs.rs`](dirs.rs)               | The three places pace keeps files: the global config file, the data directory (`PACE_HOME`) and the machine directory.                                                        |
| [`editor.rs`](editor.rs)           | Editing text in the user's editor: `VISUAL`, else `EDITOR`; the round trip that opens a file again until its check takes it; and what pace says once it saved a catalog file. |
| [`lib.rs`](lib.rs)                 | pace: a local-first command-line time tracker for work and daily life.                                                                                                        |
| [`origin.rs`](origin.rs)           | Where a setting or a directory comes from.                                                                                                                                    |
| [`output.rs`](output.rs)           | How commands describe what they did: the sentences of the tracking commands, lists in a sentence and tables.                                                                  |
| [`print.rs`](print.rs)             | Writing what pace says to the standard output, as an error instead of a panic.                                                                                                |
| [`project.rs`](project.rs)         | The project context: which project the working directory belongs to.                                                                                                          |
| [`prompt.rs`](prompt.rs)           | Asking the user, only when a terminal is attached: the `Ask` trait and the `Terminal` that the edge finds once.                                                               |
| [`questions.rs`](questions.rs)     | The question machine: a `Flow` gives questions and takes answers without I/O, and `ask_flow` drives it through the asker.                                                     |
| [`select.rs`](select.rs)           | Choosing the entry a command acts on: by its ID or short ID, `--last`, or the open entry.                                                                                     |
| [`session.rs`](session.rs)         | What a tracking command works with: the tracker over the files backend, with this machine's device ID, the clock and the settings.                                            |
| [`bin/`](bin/)                     | The `pace` binary.                                                                                                                                                            |
| [`commands/`](commands/)           | The command line: the entry point and its subcommands.                                                                                                                        |
| [`dirs/`](dirs/)                   | The three places pace keeps files: the global config file, the data directory (`PACE_HOME`) and the machine directory.                                                        |
| [`questions/`](questions/)         | The question machine: a `Flow` gives questions and takes answers without I/O, and `ask_flow` drives it through the asker.                                                     |

<!-- /files -->

## How it connects

<!-- uses -->

The crate root declares the modules. Each folder's README names what it uses.

<!-- /uses -->
