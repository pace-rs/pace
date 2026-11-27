# `questions`

The flows of the question machine in `questions.rs`, one file per command that
asks. Each flow starts with the values its command's flags gave, asks for the
rest, and ends with the field edits of the file it writes.

## Files

<!-- files -->

| File                               | What it holds                                                                                                                                                   |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`add_client.rs`](add_client.rs)   | The questions of `pace client add`: the client's name, currency, hourly rate and its first day, and the rounding of its billable time.                          |
| [`add_project.rs`](add_project.rs) | The questions of `pace project add`: the project's name, client, role and billable flag, each with a default; its budget, days and result come from flags only. |
| [`add_role.rs`](add_role.rs)       | The questions of `pace role add`: the role's name, kind, purpose and weekly target, each with a default.                                                        |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`prompt`](../prompt.rs).
- Used by: [`commands`](../commands/).

<!-- /uses -->
