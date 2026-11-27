# `report`

The parts of `report.rs`: the filters, the split of an entry's time at midnight,
the time parts, the breaks, the comparison with the previous period of `reflect
--compare`, and the revenue, which sums the charges of `billing.rs` per project,
client and currency. `report.rs` holds the `Report`, what `Report::build` takes,
and the warnings, and re-exports these values.

## Files

<!-- files -->

| File                       | What it holds                                                                                                                                                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [`breaks.rs`](breaks.rs)   | The breaks of a report: holds by source with their most frequent reasons, pauses by kind, and the catches.                                                   |
| [`compare.rs`](compare.rs) | The comparison of `reflect --compare`: each number of a report beside its value in the previous period, with the change, and the highlights of both periods. |
| [`filter.rs`](filter.rs)   | The filters of a report: projects, roles, clients, tags and devices; several of one kind combine with "or", different kinds with "and".                      |
| [`revenue.rs`](revenue.rs) | The revenue of a report: the billable time and its amount per project and per client, the totals per currency, and the time without a rate.                  |
| [`split.rs`](split.rs)     | The split of an entry's time: the stretches it was worked, outside its holds and pauses, cut to the period and at midnight in the report's zone.             |
| [`time.rs`](time.rs)       | The time parts of a report: net time per role, project and day, the back-flow, targets against actual time, budgets and who decided.                         |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: no other module.
- Used by: [`bill`](../bill/), [`billing`](../billing.rs).

<!-- /uses -->
