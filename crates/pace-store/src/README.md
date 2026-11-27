# `pace-store`

The event store backends; so far the `files` backend, one TOML event log per
device in `PACE_HOME`. `files.rs` appends and reads under a lock, `format.rs`
writes and reads the event files, `index.rs` keeps the machine-local index of
them, and `machine.rs` the device ID and the cursors. `toml_file.rs` reads
pace's TOML files with spans, so an error points at its line. `testing.rs`,
behind the feature `testing`, gives tests a scratch directory in memory.

## Files

<!-- files -->

| File                           | What it holds                                                                     |
| ------------------------------ | --------------------------------------------------------------------------------- |
| [`files.rs`](files.rs)         | The `files` backend: TOML event files in `PACE_HOME`.                             |
| [`format.rs`](format.rs)       | The TOML format of event files.                                                   |
| [`index.rs`](index.rs)         | The machine-local index of the event files.                                       |
| [`lib.rs`](lib.rs)             | The event store backends of pace, and its TOML file reader.                       |
| [`machine.rs`](machine.rs)     | The machine directory: this machine's device ID and how far it wrote its own log. |
| [`split.rs`](split.rs)         | How a device's event log is split into files: by year, month, week or day.        |
| [`testing.rs`](testing.rs)     | A scratch directory for tests, where the `files` backend's syncs cost nothing.    |
| [`toml_file.rs`](toml_file.rs) | pace's TOML files, read with spans so every error can point at its line.          |
| [`index/`](index/)             | The machine-local index of the event files.                                       |

<!-- /files -->

## How it connects

<!-- uses -->

The crate root declares the modules. Each folder's README names what it uses.

<!-- /uses -->
