# `files`

The parts of the `files` backend that rewrite this device's own log:
`own_log.rs` splits it by another period and takes the events of forgotten
entries out of it.

## Files

<!-- files -->

| File                       | What it holds                                                                                      |
| -------------------------- | -------------------------------------------------------------------------------------------------- |
| [`own_log.rs`](own_log.rs) | The rewrites of this device's own log: a new split, and the events of forgotten entries taken out. |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`disk`](../disk.rs), [`format`](../format.rs), [`index`](../index/).
- Used by: no other module.

<!-- /uses -->
