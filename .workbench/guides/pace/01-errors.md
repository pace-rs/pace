# Errors

pace's errors follow `rustic_core`'s design: `crates/pace-error`, described in
`dev-docs/src/development/errors.md`.

1. Every error is a `PaceError`: `PaceError::new(kind, "guidance")`, or
   `PaceError::with_source(kind, "guidance", error)` when a lower error caused
   it.
2. Each `{placeholder}` of the guidance gets its value with `attach_context`.
   ```rust
   PaceError::new(ErrorKind::Time, "pace doesn't know the time zone {zone}")
       .attach_context("zone", name.clone())
   ```
3. An error says how to go on with `attach_hint`: the command or the flag that
   helps.
4. Messages and hints are lowercase and end without a period.
5. Nothing to do, such as nothing running or a second hold, exits with
   `ExitCode::NothingToDo` (3).
6. Wrong input on the command line exits with `ExitCode::Usage` (2), as clap
   does.
7. Every other error exits with 1; a bug is an `ErrorKind::Internal` with
   `ask_report()`.
8. An error about a file points at its line: read pace's TOML files with
   `pace_store::toml_file`, which keeps the spans.
9. A message names the value it refuses, so the user sees what to change and a
   search for the message finds the code.
