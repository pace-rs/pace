# `testing`

`testing.rs` holds the scratch directory, the in-memory store and `Raced`, a
store whose appends race a rival tracker. Two parts have their own files:
`contract.rs` holds the checks of the `EventStore` contract, which every backend
passes, and `tests.rs` the tests of the scratch directory and the memory store.

## Files

<!-- files -->

| File                         | What it holds                                                                                                          |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| [`contract.rs`](contract.rs) | The contract of `EventStore`, as checks that every backend runs.                                                       |
| [`tests.rs`](tests.rs)       | Tests: The scratch directory lies in memory, and the memory store refuses to read storage that a panicked thread left. |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: no other module.
- Used by: [`field_edit`](../field_edit/), [`files`](../files/),
  [`renames`](../renames.rs).

<!-- /uses -->
