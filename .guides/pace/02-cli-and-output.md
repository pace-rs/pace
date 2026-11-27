# CLI and output

1. Commands are speaking, mindful verbs: `begin`, `hold`, `pause`, `catch`,
   `forget`, `store split`; `settings` is folded into `config`.
2. Flags explain themselves: no generic `--yes` or `--force`, but `--for-good`
   or `--allow-overlap`.
3. A command says in one sentence what it did, with the entry's short ID, note
   and time (`src/output.rs`): `Began ad5e "Write offer" at 09:00.`
4. Without a terminal pace never waits for input: it takes a default and says
   so, or stops and names the flag to use (`prompt::Ask::can_ask`).
5. Exit codes are 0 done, 1 error, 2 wrong usage, 3 nothing to do.
6. Bare `pace` shows `now` and a help line, with exit code 0.
7. Output for scripts is `--json`, with a `version` field.
8. The CLI resolves the project, short IDs, `--last` and `--at` before it hands
   a command to the core.
9. Abscissa's `#[derive(Command)]` needs Abscissa's `Command` trait in scope, so
   the core's comes in under another name.
   ```rust
   use pace_core::Command as UseCase;
   ```
10. Never run `pace` by hand without `PACE_HOME`, `PACE_MACHINE_DIR` and
    `PACE_CONFIG` in a scratch directory (`mktemp -d`); the file that
    `PACE_CONFIG` names must exist, and may be empty.
