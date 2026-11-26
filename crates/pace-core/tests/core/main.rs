//! Behaviour of `pace_core`.

mod commands;
mod events;
mod ids;
mod keys;

type TestResult = Result<(), Box<dyn std::error::Error>>;
