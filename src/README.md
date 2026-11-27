# `pace`

The app crate `pace-rs`: the Abscissa application and the `pace` command line on
top of the core. `application.rs` resolves the directories, the settings and the
project before a command runs. It is the only place that reads the environment:
the terminal, the editor, the hostname and `PACE_TEST_NOW` reach the commands in
the `Context`. `commands/` holds the commands, and `session.rs` opens the
tracker over the `files` backend. The commands write what the user reads through
`print.rs`; `output.rs` words the sentences of the commands, and `prompt.rs`
asks the user. A command that adds something asks through the question machine
in `questions.rs`: its flow gives the questions without I/O. `setup.rs` is the
step machine of `pace setup`, which strings these flows into a guided tour and
ends with a summary of the changes. `render.rs` draws the tables of `reflect`
and `bill`, and their rows in the schema of JSON and CSV. `doctor.rs` runs the
checks of `pace doctor` on pace's files, then hands what they read to the data
checks of `pace_core`.

## Files

<!-- files -->

| File                               | What it holds                                                                                                                                                                               |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`application.rs`](application.rs) | The Abscissa application.                                                                                                                                                                   |
| [`clock.rs`](clock.rs)             | The adapters for the `Clock` and `Ids` ports.                                                                                                                                               |
| [`commands.rs`](commands.rs)       | The command line: the entry point and its subcommands.                                                                                                                                      |
| [`config.rs`](config.rs)           | Settings and the layers they come from.                                                                                                                                                     |
| [`dirs.rs`](dirs.rs)               | The three places pace keeps files: the global config file, the data directory (`PACE_HOME`) and the machine directory.                                                                      |
| [`doctor.rs`](doctor.rs)           | The checks of `pace doctor`: each finding with its severity, its area, where it is and how to fix it.                                                                                       |
| [`editor.rs`](editor.rs)           | Editing text in the user's editor: `VISUAL`, else `EDITOR`; the round trip that opens a file again until its check takes it; and what pace says once it saved a catalog file.               |
| [`lib.rs`](lib.rs)                 | pace: a local-first command-line time tracker for work and daily life.                                                                                                                      |
| [`origin.rs`](origin.rs)           | Where a setting or a directory comes from.                                                                                                                                                  |
| [`output.rs`](output.rs)           | How commands describe what they did: the sentences of the tracking commands, lists in a sentence and tables.                                                                                |
| [`print.rs`](print.rs)             | Writing what pace says to the standard output, as an error instead of a panic.                                                                                                              |
| [`project.rs`](project.rs)         | The project context: which project the working directory belongs to.                                                                                                                        |
| [`prompt.rs`](prompt.rs)           | Asking the user, only when a terminal is attached: the `Ask` trait and the `Terminal` that the edge finds once.                                                                             |
| [`questions.rs`](questions.rs)     | The question machine: a `Flow` gives questions and takes answers without I/O, and `ask_flow` drives it through the asker.                                                                   |
| [`render.rs`](render.rs)           | Output drawn by code: the canvas a table fits, bars and headings, the tables of the report and the bill, their rows in the schema of JSON and CSV, and both from a Tera template.           |
| [`select.rs`](select.rs)           | Choosing the entry a command acts on: by its ID or short ID, `--last`, or the open entry.                                                                                                   |
| [`session.rs`](session.rs)         | What a tracking command works with: the tracker over the files backend, with this machine's device ID, the clock and the settings.                                                          |
| [`setup.rs`](setup.rs)             | The guided tour of `pace setup`: a step machine without I/O that asks for the settings, the roles, the clients and this directory's project, and ends with the field edits of what changed. |
| [`bin/`](bin/)                     | The `pace` binary.                                                                                                                                                                          |
| [`commands/`](commands/)           | The command line: the entry point and its subcommands.                                                                                                                                      |
| [`config/`](config/)               | Settings and the layers they come from.                                                                                                                                                     |
| [`dirs/`](dirs/)                   | The three places pace keeps files: the global config file, the data directory (`PACE_HOME`) and the machine directory.                                                                      |
| [`doctor/`](doctor/)               | The checks of `pace doctor`: each finding with its severity, its area, where it is and how to fix it.                                                                                       |
| [`questions/`](questions/)         | The question machine: a `Flow` gives questions and takes answers without I/O, and `ask_flow` drives it through the asker.                                                                   |
| [`render/`](render/)               | Output drawn by code: the canvas a table fits, bars and headings, the tables of the report and the bill, their rows in the schema of JSON and CSV, and both from a Tera template.           |
| [`setup/`](setup/)                 | The guided tour of `pace setup`: a step machine without I/O that asks for the settings, the roles, the clients and this directory's project, and ends with the field edits of what changed. |

<!-- /files -->

## How it connects

<!-- uses -->

The crate root declares the modules. Each folder's README names what it uses.

<!-- /uses -->
