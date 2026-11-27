//! Behaviour of `pace_core`.

mod commands;
mod events;
mod hlc;
mod ids;
mod invariants;
mod keys;
mod rules;
mod state;
mod steps;
mod tracker;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The config of the property tests: a fixed seed, so a case that fails
/// fails again on the next run and on every machine.
fn fixed_seed() -> proptest::test_runner::Config {
    proptest::test_runner::Config {
        rng_seed: proptest::test_runner::RngSeed::Fixed(20_261_126),
        ..proptest::test_runner::Config::default()
    }
}
