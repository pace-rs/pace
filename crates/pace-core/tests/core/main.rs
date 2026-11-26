//! Behaviour of `pace_core`.

mod commands;
mod events;
mod hlc;
mod ids;
mod keys;
mod rules;
mod state;
mod tracker;

type TestResult = Result<(), Box<dyn std::error::Error>>;
