# `field_edit`

The parts of the field writer in `field_edit.rs`. `table.rs` handles whole
tables: it shows a table alone, for the editor, and puts an edited table back
where the old one stood.

## Files

<!-- files -->

| File                   | What it holds                                                                                                                        |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| [`table.rs`](table.rs) | Whole tables for the field writer: a table alone as its own TOML text, and its replacement by such a text, in its place in the file. |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`toml_file`](../toml_file.rs).
- Used by: no other module.

<!-- /uses -->
