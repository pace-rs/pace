# `testing`

The parts of `testing.rs` that a store backend runs against itself. `testing.rs`
holds the test clock and the in-memory store; `contract.rs` holds the checks of
the `EventStore` contract, which every backend passes.

## Files

<!-- files -->

| File                         | What it holds                                                    |
| ---------------------------- | ---------------------------------------------------------------- |
| [`contract.rs`](contract.rs) | The contract of `EventStore`, as checks that every backend runs. |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`command`](../command.rs).
- Used by: no other module.

<!-- /uses -->
