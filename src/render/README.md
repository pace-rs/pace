# `render`

The parts of `render.rs`, which holds the `Canvas`: the width a line may fill,
and whether it may use colour and block characters. `reflect` draws its table
here in code, not from a template. The table lists the parts of the report in
the plan's order and draws bars, scaled to the largest value of a part.

`schema.rs` holds the schema of JSON and CSV, version 1: the `Column` names in
one place, the columns of each `SchemaKind`, and the writers `schema_json` and
`schema_csv`. `report_rows.rs` turns the report into rows of that schema, one
row per line of the report, each with its `part` and `group`.

`report_template.rs` renders the report from a Tera template, for the formats
`text`, `markdown` and `html`. The built-in templates live in `templates/` in
pace's root, embedded with `include_str!`; a `ReportTemplate::File` is a
template of the user's own. A template receives the JSON document of the schema,
`SchemaDocument`.

With `--compare`, `report_compare.rs` words the `ReportComparison` of the core:
the cells `was 3h`, `+2h 15m` and `+75%` beside a number of the table, the
signed changes of the rows, and the highlights. A row of the schema holds the
cells of the previous period apart; JSON writes them as the object `previous`,
CSV under the columns `previous_<column>`.

## Files

<!-- files -->

| File                                       | What it holds                                                                                                                                       |
| ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`report_compare.rs`](report_compare.rs)   | The comparison of `reflect --compare` in words: the previous value and the change beside a number, for the table and the rows, and the highlights.  |
| [`report_rows.rs`](report_rows.rs)         | The report of `reflect` as rows of the schema, for `--format json` and `--format csv`: one row per line of the report, each under its part.         |
| [`report_table.rs`](report_table.rs)       | The report of `reflect` as a table for the terminal: its parts in the plan's order, with bars for the time per role, project and day.               |
| [`report_template.rs`](report_template.rs) | The report of `reflect` from a Tera template, for `--format text`, `markdown` and `html`: the built-in templates and the reader of a template file. |
| [`schema.rs`](schema.rs)                   | The schema of pace's JSON and CSV, version 1: the column names in one place, and the writers of both formats.                                       |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`config`](../config/), [`output`](../output.rs).
- Used by: [`commands`](../commands/).

<!-- /uses -->
