//! Behaviour of `pace_core`.

mod catalog;
mod commands;
mod hlc;
mod ids;
mod keys;
mod money;
mod report;
mod rules;
mod state;
mod steps;
mod texts;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The config of the property tests: a fixed seed, so a case that fails
/// fails again on the next run and on every machine.
fn fixed_seed() -> proptest::test_runner::Config {
    proptest::test_runner::Config {
        rng_seed: proptest::test_runner::RngSeed::Fixed(20_261_126),
        ..proptest::test_runner::Config::default()
    }
}
