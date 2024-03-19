//! Pace
//!
//! Application based on the [Abscissa] framework.
//!
//! [Abscissa]: https://github.com/iqlusioninc/abscissa

// Tip: Deny warnings with `RUSTFLAGS="-D warnings"` environment variable in CI

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    trivial_casts,
    unused_lifetimes,
    unused_qualifications
)]

pub mod application;
pub mod commands;
pub mod error;

// Re-export pace libraries
pub use pace_cli;
pub use pace_core;

pub mod prelude {
    //! Application-local prelude: conveniently import types/functions/macros
    //! which are generally useful and should be available in every module with
    //! `use crate::prelude::*;`

    /// Abscissa core prelude
    pub use abscissa_core::prelude::*;

    /// Application state
    pub use crate::application::PACE_APP;
}
