//! Behaviour of `pace_time`.

mod durations;
mod frames;

type TestResult = Result<(), Box<dyn std::error::Error>>;
