//! Main entry point for PaceRs

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use pace_rs::application::APP;

/// Boot PaceRs
fn main() {
    abscissa_core::boot(&APP);
}
