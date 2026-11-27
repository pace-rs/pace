# `pace-core`

The domain of pace, without I/O: entries and their events, the state built from
them, the rules that decide what a command records, and the ports for the clock,
IDs and the event store. `tracker.rs` runs a command: it reads the state through
the `EventStore` port, lets `rules.rs` decide and appends the events. The
`testing` feature adds `testing.rs` for the tests of other crates.

## Files

<!-- files -->

| File                         | What it holds                                                                                                                         |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| [`command.rs`](command.rs)   | The use cases as data; see `Command`.                                                                                                 |
| [`entry.rs`](entry.rs)       | Entries: a stretch of tracked time with its note, project and tags, its holds, pauses and catches, and what it is doing now.          |
| [`event.rs`](event.rs)       | The events of the log: what happened to an entry or a batch, as the core decided it.                                                  |
| [`id.rs`](id.rs)             | The IDs of events, batches, devices and entries, each a UUIDv7.                                                                       |
| [`key.rs`](key.rs)           | The names people type: project keys such as `acme-webshop`, and tags.                                                                 |
| [`lib.rs`](lib.rs)           | The domain of pace: its rules, ports and commands.                                                                                    |
| [`port.rs`](port.rs)         | The ports of the core: the clock, the source of IDs, the event store and the notifier.                                                |
| [`rules.rs`](rules.rs)       | What each command turns into: the events of one batch, or an error that says how to fix it.                                           |
| [`short_id.rs`](short_id.rs) | Short IDs: the last characters of an entry ID that tell it apart.                                                                     |
| [`state.rs`](state.rs)       | The state of the entries, built from the events in merged order.                                                                      |
| [`testing.rs`](testing.rs)   | For tests of pace and of store backends: a settable clock, an in-memory `EventStore`, and the store contract every backend must keep. |
| [`tracker.rs`](tracker.rs)   | The tracker: runs a command over the ports and says what it did.                                                                      |
| [`tracking.rs`](tracking.rs) | Where a hold came from, and what kind of break a pause is.                                                                            |
| [`rules/`](rules/)           | What each command turns into: the events of one batch, or an error that says how to fix it.                                           |

<!-- /files -->

## How it connects

<!-- uses -->

The crate root declares the modules. Each folder's README names what it uses.

<!-- /uses -->
