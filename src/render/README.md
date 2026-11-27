# `render`

The parts of `render.rs`, which holds the `Canvas`: the width a line may fill,
and whether it may use colour and block characters. `reflect` draws its table
here in code, not from a template. The table lists the parts of the report in
the plan's order and draws bars, scaled to the largest value of a part.

## Files

<!-- files -->

| File                                 | What it holds                                                                                                                         |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------- |
| [`report_table.rs`](report_table.rs) | The report of `reflect` as a table for the terminal: its parts in the plan's order, with bars for the time per role, project and day. |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`output`](../output.rs).
- Used by: [`commands`](../commands/).

<!-- /uses -->
