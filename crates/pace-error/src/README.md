# `pace-error`

One error type for all of pace: `PaceError`, with a kind, guidance with
`{placeholders}`, context, a hint, an exit code and an optional snippet of the
input. It reports through miette.

## Files

<!-- files -->

| File               | What it holds                                  |
| ------------------ | ---------------------------------------------- |
| [`lib.rs`](lib.rs) | Errors with fix hints and exit codes for pace. |

<!-- /files -->

## How it connects

<!-- uses -->

The crate root declares the modules. Each folder's README names what it uses.

<!-- /uses -->
