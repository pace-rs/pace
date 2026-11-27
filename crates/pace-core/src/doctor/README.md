# `doctor`

The data checks of `pace doctor`, one file per kind. `entries.rs` finds several
open entries, overlaps that nothing allows and project keys without a record;
`conflicts.rs` finds the fields that two devices changed. `structure.rs` checks
the catalog, `rates.rs` the billed and billable entries against the rates.
`doctor.rs` holds the `DataFinding` and runs the checks in order.

## Files

<!-- files -->

| File                           | What it holds                                                                                                                                                      |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [`conflicts.rs`](conflicts.rs) | The check of `pace doctor` for fields that two devices changed: per field, the later change by HLC counts, and doctor names every value.                           |
| [`entries.rs`](entries.rs)     | The entry checks of `pace doctor`: several open entries, overlaps that nothing allows, fields changed on two devices, and entries whose project key has no record. |
| [`rates.rs`](rates.rs)         | The billing checks of `pace doctor`: billed entries whose amount would differ with today's rates, and billable entries before the first rate of their client.      |
| [`structure.rs`](structure.rs) | The structure checks of `pace doctor`: billable projects without a client, and weekly targets above the 168 hours of a week.                                       |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`bill`](../bill/), [`report`](../report/).
- Used by: no other module.

<!-- /uses -->
