//! Behaviour of `pace_store`.

mod device;
mod files;

type TestResult = Result<(), Box<dyn std::error::Error>>;
