# `pace-time`

Time handling on jiff: the `--at` grammar, durations, report periods and time
ranges, and how pace shows durations and relative moments. Every moment is a
`ZonedDateTime`.

## Files

<!-- files -->

| File                         | What it holds                                                                                           |
| ---------------------------- | ------------------------------------------------------------------------------------------------------- |
| [`at.rs`](at.rs)             | The `--at` grammar: `09:30`, `yesterday 17:00`, `2026-11-26 09:00`, `-15m`.                             |
| [`display.rs`](display.rs)   | Durations and moments as pace shows them: `1h 5m`, `25 minutes ago`.                                    |
| [`duration.rs`](duration.rs) | The time between two moments, and durations as people type them.                                        |
| [`frame.rs`](frame.rs)       | Periods for reports: named frames such as `today` and `last-week`, or the days from `--from` to `--to`. |
| [`lib.rs`](lib.rs)           | Time handling for pace, on jiff.                                                                        |
| [`range.rs`](range.rs)       | Half-open ranges of time, `[start, end)`, whose bounds may be open.                                     |

<!-- /files -->

## How it connects

<!-- uses -->

The crate root declares the modules. Each folder's README names what it uses.

<!-- /uses -->
