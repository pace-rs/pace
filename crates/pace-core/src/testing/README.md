# `testing`

`testing.rs` holds the test clock and the in-memory store. Two parts have their
own files: `contract.rs` holds the checks of the `EventStore` contract, which
every backend passes, and `tests.rs` the tests of the test doubles themselves.

## Files

<!-- files -->

| File                         | What it holds                                                               |
| ---------------------------- | --------------------------------------------------------------------------- |
| [`contract.rs`](contract.rs) | The contract of `EventStore`, as checks that every backend runs.            |
| [`tests.rs`](tests.rs)       | Tests: The test doubles refuse to read a value that a panicked thread left. |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`command`](../command.rs).
- Used by: no other module.

<!-- /uses -->
