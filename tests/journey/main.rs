mod cli;
mod hold_resume;
mod start_finish_different_time_zone;

// DEBUGGING: To make life easier, you can import SimpleLogger from simplelog crate
//
// `use simplelog::{Config, SimpleLogger};`
//
// and use it like this to initialize the logger in your tests:
//
// `SimpleLogger::init(tracing::log::LevelFilter::Debug, Config::default())?;`
