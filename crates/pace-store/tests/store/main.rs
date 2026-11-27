//! Behaviour of `pace_store`.

mod device;
mod devices;
mod failures;
mod files;

type TestResult = Result<(), Box<dyn std::error::Error>>;
