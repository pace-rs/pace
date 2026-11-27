# `format`

The parts of `format.rs`: `event_type.rs` names each event type as the log does
and lists the fields it allows, `writing.rs` turns an event into its `[[event]]`
record, and `reading.rs` reads a record back into an event, checked against its
type. `tests.rs` holds their tests: every event kind reads back as written, and
each invalid event file is refused with its reason.

## Files

<!-- files -->

| File                             | What it holds                                                                                          |
| -------------------------------- | ------------------------------------------------------------------------------------------------------ |
| [`event_type.rs`](event_type.rs) | The event types as the log names them, and the fields each allows.                                     |
| [`reading.rs`](reading.rs)       | An `[[event]]` record read back as an event, checked against its type.                                 |
| [`tests.rs`](tests.rs)           | Tests: Every event kind reads back as written, and each invalid event file is refused with its reason. |
| [`writing.rs`](writing.rs)       | An event written as its `[[event]]` record.                                                            |

<!-- /files -->

## How it connects

<!-- uses -->

- Uses: [`hlc`](../hlc.rs).
- Used by: [`files`](../files/).

<!-- /uses -->
