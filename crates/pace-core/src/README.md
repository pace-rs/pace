# `pace-core`

The domain of pace, without I/O: entries and their events, the state built from
them, the rules that decide what a command records, and the traits for the clock
and the IDs. `steps.rs` holds a command's step machine: it names what to read,
lets `rules.rs` decide and says what the append means, without I/O. The event
store and the driver of the machine live in `pace_store`. The `testing` feature
adds `testing.rs` for the tests of other crates: a settable clock, deterministic
IDs and the lock the test doubles share.

## Files

<!-- files -->

| File                         | What it holds                                                                                                                          |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| [`command.rs`](command.rs)   | The use cases as data; see `Command`.                                                                                                  |
| [`entry.rs`](entry.rs)       | Entries: a stretch of tracked time with its note, project and tags, its holds, pauses and catches, and what it is doing now.           |
| [`event.rs`](event.rs)       | The events of the log: what happened to an entry or a batch, as the core decided it.                                                   |
| [`id.rs`](id.rs)             | The IDs of events, batches, devices and entries, each a UUIDv7.                                                                        |
| [`key.rs`](key.rs)           | The names people type: project keys such as `acme-webshop`, and tags.                                                                  |
| [`lib.rs`](lib.rs)           | The domain of pace: its rules, its step machine and its commands.                                                                      |
| [`open.rs`](open.rs)         | The open entry a command acts on, and how pace names entries in its texts: the short ID, the note and the times.                       |
| [`port.rs`](port.rs)         | The ports of the core: the clock, the source of IDs and the notifier.                                                                  |
| [`rules.rs`](rules.rs)       | What each command turns into: the events of one batch, or an error that says how to fix it.                                            |
| [`scope.rs`](scope.rs)       | What a read of the event log must return: the entries of a time range, the open ones, and those with an event recorded since a moment. |
| [`short_id.rs`](short_id.rs) | Short IDs: the last characters of an entry ID that tell it apart.                                                                      |
| [`state.rs`](state.rs)       | The state of the entries, built from the events in merged order.                                                                       |
| [`steps.rs`](steps.rs)       | The step machine of a command: what to read, the batch to append, and what the append means, without I/O.                              |
| [`testing.rs`](testing.rs)   | For tests of pace and of store backends: a settable clock, deterministic IDs, and the lock that the test doubles share.                |
| [`text.rs`](text.rs)         | Text that people write: the note of an entry, the reason of a hold or a pause, a caught thought.                                       |
| [`tracking.rs`](tracking.rs) | Where a hold came from, and what kind of break a pause is.                                                                             |
| [`rules/`](rules/)           | What each command turns into: the events of one batch, or an error that says how to fix it.                                            |
| [`testing/`](testing/)       | For tests of pace and of store backends: a settable clock, deterministic IDs, and the lock that the test doubles share.                |

<!-- /files -->

## How it connects

<!-- uses -->

The crate root declares the modules. Each folder's README names what it uses.

<!-- /uses -->
