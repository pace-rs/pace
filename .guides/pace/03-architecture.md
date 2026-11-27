# Architecture

1. `pace_core` has no I/O: the clock, the IDs, the event store and the notifier
   are ports (`crates/pace-core/src/port.rs`); the app and the backend crates
   hold the adapters.
2. A command is a serializable value (`pace_core::command`); the rules
   (`rules.rs`) turn it into the events of one batch, or into an error that says
   how to fix it.
3. Every timestamp is a jiff `ZonedDateTime`; time handling lives in
   `pace_time`.
4. What happens in time is an event in one append-only TOML log per device in
   `PACE_HOME`; structure (roles, clients, projects, goals, task definitions)
   stays plain TOML state files.
5. The ports block: `EventStore`'s methods return their `PaceResult` directly,
   and a backend does its I/O before it returns.
6. pace starts no async runtime. The SQL backends bring an async store trait and
   a driver behind a feature; then Abscissa starts one runtime for the whole
   program.
7. The machine directory holds what is never synced (device ID, index, cursors,
   lock); `PACE_HOME` holds what syncs.
8. A setting comes from layers (global file, manifest, environment, flag) with
   its origin (`src/config.rs`, `pace config show --origin`); an unknown key is
   an error.
9. The project comes from the working directory: `.pace.toml` or `pace.toml`
   (with `.pace/`), else `[project-paths]`; the global config is
   `pace.global.toml`.
