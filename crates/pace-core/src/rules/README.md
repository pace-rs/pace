# `rules`

The parts of `rules.rs`. `checks.rs` checks times against an entry: that an
entry has a length, that its holds and pauses lie inside it, and that a break
begins after the one before. `marking.rs` holds the rule of `bill
--mark-billed`, which freezes a bill as one `billing.marked` per entry.
`billed.rs` protects a billed entry: without `--change-billed`, `adjust`,
`forget` and an `undo` that touches it are refused. `rules.rs` decides what a
command records and calls all three.

## Files

<!-- files -->

| File                       | What it holds                                                                                                                                                                            |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`billed.rs`](billed.rs)   | The protection of billed entries: without `--change-billed`, the rules refuse to adjust or forget a billed entry, and to undo a batch that changed one, except the batch that marked it. |
| [`checks.rs`](checks.rs)   | The checks of times against an entry: its length, its breaks, and a moment before another one.                                                                                           |
| [`marking.rs`](marking.rs) | The rule of `bill --mark-billed`: the bill of a client, decided on the state as it is, frozen as one `billing.marked` per entry.                                                         |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`command`](../command.rs).
- Used by: no other module.

<!-- /uses -->
