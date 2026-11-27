# `rules`

The parts of `rules.rs`. `checks.rs` checks times against an entry: that an
entry has a length, that its holds and pauses lie inside it, and that a break
begins after the one before. `marking.rs` holds the rule of `bill
--mark-billed`, which freezes a bill as one `billing.marked` per entry.
`rules.rs` decides what a command records and calls both.

## Files

<!-- files -->

| File                       | What it holds                                                                                                                    |
| -------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| [`checks.rs`](checks.rs)   | The checks of times against an entry: its length, its breaks, and a moment before another one.                                   |
| [`marking.rs`](marking.rs) | The rule of `bill --mark-billed`: the bill of a client, decided on the state as it is, frozen as one `billing.marked` per entry. |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`command`](../command.rs).
- Used by: no other module.

<!-- /uses -->
