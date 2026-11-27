# `rules`

The parts of `rules.rs` that check times against an entry: that an entry has a
length, that its holds and pauses lie inside it, and that a break begins after
the one before. `rules.rs` decides what a command records and calls these
checks.

## Files

<!-- files -->

| File                     | What it holds                                                                                  |
| ------------------------ | ---------------------------------------------------------------------------------------------- |
| [`checks.rs`](checks.rs) | The checks of times against an entry: its length, its breaks, and a moment before another one. |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: no other module.
- Used by: no other module.

<!-- /uses -->
