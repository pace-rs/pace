# `files`

The parts of the `files` backend that write this device's own log: `own_log.rs`
splits it by another period and takes the events of forgotten entries out of it,
in the write of the batch that forgets them and in the files left over;
`renames.rs` puts a batch into its event files, all or none. `parsed.rs` keeps
the events of each file text that this process parsed, so one command parses the
current month's file once.

## Files

<!-- files -->

| File                       | What it holds                                                                                      |
| -------------------------- | -------------------------------------------------------------------------------------------------- |
| [`own_log.rs`](own_log.rs) | The rewrites of this device's own log: a new split, and the events of forgotten entries taken out. |
| [`parsed.rs`](parsed.rs)   | The events of the event files this process parsed, by file and text.                               |
| [`renames.rs`](renames.rs) | The renames that put a batch into its event files, all or none.                                    |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`disk`](../disk.rs), [`format`](../format/), [`index`](../index/),
  [`machine`](../machine.rs), [`testing`](../testing/).
- Used by: no other module.

<!-- /uses -->
