# `render`

The parts of `render.rs`, which holds the `Canvas`: the width a line may fill,
and whether it may use colour and block characters. `reflect` draws its table
here in code, not from a template. The table lists the parts of the report in
the plan's order and draws bars, scaled to the largest value of a part.

`schema.rs` holds the schema of JSON and CSV, version 1: the `Column` names in
one place, the columns of each `SchemaKind`, and the writers `schema_json` and
`schema_csv`. `report_rows.rs` turns the report into rows of that schema, one
row per line of the report, each with its `part` and `group`.

## Files

<!-- files -->

| File                                 | What it holds                                                                                                                               |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| [`report_rows.rs`](report_rows.rs)   | The report of `reflect` as rows of the schema, for `--format json` and `--format csv`: one row per line of the report, each under its part. |
| [`report_table.rs`](report_table.rs) | The report of `reflect` as a table for the terminal: its parts in the plan's order, with bars for the time per role, project and day.       |
| [`schema.rs`](schema.rs)             | The schema of pace's JSON and CSV, version 1: the column names in one place, and the writers of both formats.                               |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`config`](../config/), [`output`](../output.rs).
- Used by: [`commands`](../commands/).

<!-- /uses -->
