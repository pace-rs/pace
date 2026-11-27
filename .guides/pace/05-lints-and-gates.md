# Lints and gates

1. Before each commit, chain the checks with `&&` so a failing one blocks it;
   `just check` runs fmt, dprint, clippy, the tests and codegate's gates.
   ```sh
   cargo fmt --all -- --check && dprint check && cargo clippy --workspace \
     --all-targets --all-features -- -D warnings && cargo nextest run
   ```
2. The workspace lints are clippy's `pedantic` and `nursery`, `unwrap_used`,
   `expect_used` and rustc's `unused_results`; CI makes warnings errors. In
   tests, `clippy.toml` allows `unwrap` and `expect`.
3. Discard a value explicitly with `_ = …`.
4. An `#[allow(clippy::expect_used)]` says why; every new allowance fails
   codegate's lint-allows gate until `codegate lint-allows --bless` records it.
5. `unsafe_code` is forbidden, and `dead_code` is not allowed.
6. The pre-commit hook (`.githooks/pre-commit`) runs `codegate check`, the
   pre-push hook `codegate check --full`; after a fresh clone, `codegate setup`
   makes `.githooks` git's `core.hooksPath`.
7. A hand-written Rust file holds at most 700 production lines; split a longer
   one into modules along its own structure.
8. No function scores above CRAP 30: give it tests, or split it into named
   functions; above complexity 30 only a split helps.
9. Import an own function by its name, a name that is clear without its module
   (`short_id`, not `output::short`); no path goes through `super` but `use
   super::*;` in tests.
10. Every folder of a `src` tree has a README: write its prose, then `codegate
    readmes && dprint fmt` updates and aligns its tables.
11. Every module starts with a `//!` doc whose first sentence says what it
    holds; the README's table shows that sentence.
12. Markdown, TOML and JSON are formatted with dprint at 80 columns; run `dprint
    fmt` in the repository before a commit.
