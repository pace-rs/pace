//! Main entry point for Pace

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use pace_rs::application::PACE_APP;

/// Boot Pace
fn main() {
    abscissa_core::boot(&PACE_APP);
}
