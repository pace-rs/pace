# `setup`

The parts of the step machine of `pace setup` in `setup.rs`. Each file holds the
questions of one part that no other command asks, or the summary that ends the
tour. The roles, the clients and the project of the working directory are asked
by the flows of `questions`.

## Files

<!-- files -->

| File                                     | What it holds                                                                                                                                                                |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`rate.rs`](rate.rs)                     | The questions of a client's new rate in `pace setup`: the hourly rate, with the current one as the default, and its first day.                                               |
| [`record.rs`](record.rs)                 | The questions of a project's record in `pace setup`: its name, client, role and billable flag, each with the record's value as the default, so Enter keeps it.               |
| [`saved.rs`](saved.rs)                   | The state of `pace setup` as it saves: the step, and each answer with the question it answered and the value it replaced.                                                    |
| [`setup_file.rs`](setup_file.rs)         | The file `setup.toml` in the machine directory, where `pace setup` keeps its answers until it writes them, so an interrupted setup can continue.                             |
| [`step_questions.rs`](step_questions.rs) | The questions that the steps of `pace setup` ask themselves: the settings, whether to add a role, what to do with the clients, the offer for this directory and the summary. |
| [`summary.rs`](summary.rs)               | The summary of `pace setup`: each change it writes, as one line per file and field, such as `pace.global.toml: time-zone system → Europe/Vienna`.                            |
| [`tests.rs`](tests.rs)                   | Tests: Tests of the setup machine: each step with answers given in order, the defaults of a first and a second run, the summary and the saved state.                         |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`output`](../output.rs), [`questions`](../questions/).
- Used by: [`commands`](../commands/), [`doctor`](../doctor/).

<!-- /uses -->
