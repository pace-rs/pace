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
5. A synchronous backend implements the async ports with
   `std::future::ready(...)`.
6. Futures run on the one current-thread Tokio runtime (`block_on` in
   `src/session.rs`); tests use `pollster`.
7. The machine directory holds what is never synced (device ID, index, cursors,
   lock); `PACE_HOME` holds what syncs.
8. A setting comes from layers (global file, manifest, environment, flag) with
   its origin (`src/config.rs`, `pace config show --origin`); an unknown key is
   an error.
9. The project comes from the working directory: `.pace.toml` or `pace.toml`
   (with `.pace/`), else `[project-paths]`; the global config is
   `pace.global.toml`.
