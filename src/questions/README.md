# `questions`

The flows of the question machine in `questions.rs`, one file per command that
asks. Each flow starts with the values its command's flags gave, asks for the
rest, and ends with the field edits of the file it writes.

## Files

<!-- files -->

| File                         | What it holds                                                                                            |
| ---------------------------- | -------------------------------------------------------------------------------------------------------- |
| [`add_role.rs`](add_role.rs) | The questions of `pace role add`: the role's name, kind, purpose and weekly target, each with a default. |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`prompt`](../prompt.rs).
- Used by: [`commands`](../commands/).

<!-- /uses -->
