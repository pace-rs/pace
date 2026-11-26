//! Where a setting or a directory comes from.

use std::fmt;

/// The source of an effective value, shown by `pace config show --origin`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// pace's default.
    Default,
    /// Portable mode: next to the executable.
    Portable,
    /// An environment variable.
    Env(&'static str),
    /// A command-line flag.
    Flag(&'static str),
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default => f.write_str("default"),
            Self::Portable => f.write_str("portable mode, next to the executable"),
            Self::Env(name) | Self::Flag(name) => write!(f, "from {name}"),
        }
    }
}
