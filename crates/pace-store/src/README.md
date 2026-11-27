# `pace-store`

The event store: its port `EventStore` (`port.rs`), the `Tracker` that drives
pace_core's tracking machine over it (`tracker.rs`), and the backends; so far
the `files` backend, one TOML event log per device in `PACE_HOME`. `files.rs`
appends and reads under a lock, `format.rs` writes and reads the event files,
`index.rs` keeps the machine-local index of them, and `machine.rs` the device ID
and the cursors. `disk.rs` holds the file operations they share, `hlc.rs` the
HLC as pace's files keep it. `toml_file.rs` reads pace's TOML files with spans,
so an error points at its line. `testing.rs`, behind the feature `testing`,
gives tests a scratch directory in memory, the in-memory `MemoryStore` and
`Raced`; `testing/contract.rs` holds the store contract every backend passes.

## Files

<!-- files -->

| File                           | What it holds                                                                                                                            |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| [`disk.rs`](disk.rs)           | The store's files on disk: atomic writes, the lock, the event files of a device directory, and the error of a failed file operation.     |
| [`files.rs`](files.rs)         | The `files` backend: TOML event files in `PACE_HOME`.                                                                                    |
| [`format.rs`](format.rs)       | The TOML format of event files.                                                                                                          |
| [`hlc.rs`](hlc.rs)             | The HLC as pace's files keep it: as text in the event files, as milliseconds since the epoch and the counter in the machine directory.   |
| [`index.rs`](index.rs)         | The machine-local index of the event files.                                                                                              |
| [`lib.rs`](lib.rs)             | The event store of pace: its port, its driver, its backends, and pace's TOML file reader.                                                |
| [`machine.rs`](machine.rs)     | The machine directory: this machine's device ID and how far it wrote its own log.                                                        |
| [`port.rs`](port.rs)           | The event store port: the trait every backend implements.                                                                                |
| [`split.rs`](split.rs)         | How a device's event log is split into files: by year, month, week or day.                                                               |
| [`testing.rs`](testing.rs)     | For tests of pace and of store backends: a scratch directory, an in-memory `EventStore`, and the store contract every backend must keep. |
| [`toml_file.rs`](toml_file.rs) | pace's TOML files, read with spans so every error can point at its line.                                                                 |
| [`tracker.rs`](tracker.rs)     | The tracker: the blocking driver of the tracking machine, over the event store.                                                          |
| [`files/`](files/)             | The `files` backend: TOML event files in `PACE_HOME`.                                                                                    |
| [`index/`](index/)             | The machine-local index of the event files.                                                                                              |
| [`testing/`](testing/)         | For tests of pace and of store backends: a scratch directory, an in-memory `EventStore`, and the store contract every backend must keep. |

<!-- /files -->

## How it connects

<!-- uses -->

The crate root declares the modules. Each folder's README names what it uses.

<!-- /uses -->
