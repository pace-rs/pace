# Tests

1. Write the failing test first (unit, integration or journey test), watch it
   fail, then write the code.
2. Journey tests run the `pace` binary in a `Sandbox`
   (`tests/journey/support.rs`) and snapshot the transcript of
   `Sandbox::journey` with insta.
3. `Sandbox::pace_at("09:00")` fixes the clock with `PACE_TEST_NOW`; the IDs
   stay random.
4. A snapshot shows IDs as `[ID-1]`, `[ID-2]`, …: `Sandbox::journey` redacts its
   transcript, and a test that builds one by hand calls `Sandbox::redact_ids`.
5. A test that names an entry which doesn't exist uses `deadbeef`.
6. `INSTA_UPDATE=new cargo nextest run` writes `*.snap.new`: review the part
   after the second `---`, then rename it to `.snap` without the
   `assertion_line:` line (there is no cargo-insta).
7. A test name is a sentence without articles, as RFC 430 wants
   (`manifest_wins_over_path_mapping`); codegate's casing gate checks it.
8. Two devices are separate data and machine directories, synced by copying
   device directories (`Sandbox::sync_events_from`).
9. Core tests build a tracker in-process with `MemoryStore`, `TestClock` and
   `SequentialIds` (`pace_core::testing`), one seed per simulated device.
10. Every store backend passes
    `pace_core::testing::contract::run_store_contract`.
11. A test that runs the binary keeps `LLVM_PROFILE_FILE` (no `env_clear()`), so
    its coverage counts for the CRAP gate.
12. An interactive path needs a pty and has no journey test yet, so keep its
    logic in functions a test can call.
13. A test that writes through a store backend, or a `Sandbox`, takes its
    temporary directory from `pace_store::testing::scratch_dir`: in memory, a
    sync costs nothing, so the slow-tests gate does not time the disk.
