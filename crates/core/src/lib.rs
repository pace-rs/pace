#![forbid(unsafe_code)]
#![warn(
    // missing_docs,
    rust_2018_idioms,
    trivial_casts,
    unused_lifetimes,
    unused_qualifications
)]
#![allow(unused_imports)]
#![allow(dead_code)]

pub mod config;
pub mod domain;
pub mod error;
pub mod service;
pub mod storage;
pub mod util;

// Re-export commonly used external crates
pub use toml;
