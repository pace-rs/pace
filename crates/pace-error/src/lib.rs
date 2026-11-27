//! Errors with fix hints and exit codes for pace.
//!
//! [`PaceError`] is pace's one error type, always boxed in a [`PaceResult`].
//! It follows [`rustic_core`'s error module]. An error holds these parts:
//!
//! - a coarse [`ErrorKind`], for a caller that decides on it
//! - a guidance message, whose `{placeholders}` the
//!   [context](PaceError::attach_context) fills
//! - a [hint](PaceError::attach_hint) that says how to go on
//! - the [exit code](ExitCode) of the process, for scripts
//! - the [cause](PaceError::with_source) that led to it, kept as its `source()`
//! - a [snippet](Snippet) that points at a line of a file
//!
//! `Display` prints the filled guidance alone. The [`miette::Diagnostic`]
//! implementation adds the code, the hint, the severity and the snippet. It
//! lists the context that no placeholder took as details. miette's report
//! handler prints the causes from `source()`. An error that is a bug
//! [asks to be reported](PaceError::ask_report).
//!
//! ```
//! use pace_error::{ErrorKind, ExitCode, PaceError};
//!
//! let error = PaceError::new(ErrorKind::Other, "nothing runs in {project}")
//!     .attach_context("project", "acme")
//!     .attach_hint("`pace begin` starts an entry")
//!     .attach_exit_code(ExitCode::NothingToDo);
//!
//! assert_eq!(error.to_string(), "nothing runs in acme");
//! assert_eq!(error.hint(), Some("`pace begin` starts an entry"));
//! assert_eq!(error.exit_code(), ExitCode::NothingToDo);
//! ```
//!
//! [`rustic_core`'s error module]: https://docs.rs/rustic_core/latest/rustic_core/error/index.html

use std::{
    error::Error,
    fmt::{self, Display},
    ops::Range,
};

use miette::{Diagnostic, LabeledSpan, NamedSource, SourceCode};

/// Where a user reports a bug.
const ISSUE_URL: &str = "https://github.com/pace-rs/pace/issues/new";

/// Result type of every fallible public function in pace.
pub type PaceResult<T, E = Box<PaceError>> = Result<T, E>;

/// What an error is related to.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// the configuration
    Configuration,
    /// pace's directories
    Directories,
    /// the project context
    Project,
    /// invalid user input
    InvalidInput,
    /// input/output operations
    InputOutput,
    /// time handling
    Time,
    /// internal operations; a bug, not something the user can fix
    Internal,
    /// general operations
    Other,
}

/// How serious an error is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Informational
    Info,
    /// Warning: pace goes on
    Warning,
    /// Error: the command stops
    Error,
    /// Fatal: pace stops
    Fatal,
}

impl From<Severity> for miette::Severity {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Info => Self::Advice,
            Severity::Warning => Self::Warning,
            Severity::Error | Severity::Fatal => Self::Error,
        }
    }
}

/// The exit code of a `pace` process that an error ends, a contract with
/// scripts; a command that succeeds exits with 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// 1: an error stopped the command.
    Error,
    /// 2: the command line was wrong (clap uses the same code).
    Usage,
    /// 3: there was nothing to do, for example `pace end` while nothing runs.
    NothingToDo,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> Self {
        match code {
            ExitCode::Error => 1,
            ExitCode::Usage => 2,
            ExitCode::NothingToDo => 3,
        }
    }
}

/// A part of a file to show with an error, with a label at a span.
#[derive(Debug)]
pub struct Snippet {
    source: NamedSource<String>,
    span: Range<usize>,
    label: String,
}

impl Snippet {
    /// Point at `span` (byte offsets) of the file `name` with the content
    /// `source`, and label it.
    pub fn new(
        name: impl AsRef<str>,
        source: impl Into<String>,
        span: Range<usize>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            source: NamedSource::new(name, source.into()),
            span,
            label: label.into(),
        }
    }
}

/// An error of pace. The [crate documentation](crate) names its parts.
#[derive(Debug)]
#[non_exhaustive]
pub struct PaceError {
    /// What the error is related to.
    kind: ErrorKind,

    /// The message, with `{placeholders}` for context values.
    guidance: String,

    /// How to fix the error.
    hint: Option<String>,

    /// A stable code for the error.
    error_code: Option<String>,

    /// Whether to ask the user to report the error.
    ask_report: bool,

    /// Key-value pairs, filled into the guidance or shown as details.
    context: Vec<(String, String)>,

    /// The cause of the error.
    source: Option<Box<dyn Error + Send + Sync>>,

    /// How serious the error is.
    severity: Option<Severity>,

    /// The exit code of the process if this error ends it.
    exit_code: ExitCode,

    /// The part of a file the error is about.
    snippet: Option<Snippet>,
}

impl PaceError {
    /// Create an error of the given kind with a guidance message.
    pub fn new(kind: ErrorKind, guidance: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            kind,
            guidance: guidance.into(),
            hint: None,
            error_code: None,
            ask_report: false,
            context: Vec::new(),
            source: None,
            severity: None,
            exit_code: ExitCode::Error,
            snippet: None,
        })
    }

    /// Create an error with a guidance message and a cause.
    pub fn with_source(
        kind: ErrorKind,
        guidance: impl Into<String>,
        source: impl Into<Box<dyn Error + Send + Sync>>,
    ) -> Box<Self> {
        Self::new(kind, guidance).attach_source(source)
    }

    /// What the error is related to.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// How to fix the error, if known.
    #[must_use]
    pub fn hint(&self) -> Option<&str> {
        self.hint.as_deref()
    }

    /// The exit code of the process if this error ends it.
    #[must_use]
    pub const fn exit_code(&self) -> ExitCode {
        self.exit_code
    }

    /// Whether the error has the given code.
    #[must_use]
    pub fn is_code(&self, code: &str) -> bool {
        self.error_code.as_deref() == Some(code)
    }

    /// A context value by key, if present.
    #[must_use]
    pub fn context_value(&self, key: &str) -> Option<&str> {
        self.context
            .iter()
            .find_map(|(k, v)| (k == key).then_some(v.as_str()))
    }

    /// The guidance with its placeholders filled, and the context entries
    /// that were not used for a placeholder.
    ///
    /// It reads the template once, so a value that holds `{key}` stays as
    /// written. A placeholder without a context entry stays as written too.
    fn filled_guidance(&self) -> (String, Vec<&(String, String)>) {
        let mut filled = String::with_capacity(self.guidance.len());
        let mut used = Vec::new();
        let mut rest = self.guidance.as_str();
        while let Some((before, after)) = rest.split_once('{') {
            filled.push_str(before);
            let placeholder = after
                .split_once('}')
                .and_then(|(key, tail)| Some((self.context_entry(key)?, tail)));
            if let Some(((index, value), tail)) = placeholder {
                filled.push_str(value);
                used.push(index);
                rest = tail;
            } else {
                filled.push('{');
                rest = after;
            }
        }
        filled.push_str(rest);
        let unused = self
            .context
            .iter()
            .enumerate()
            .filter(|(index, _)| !used.contains(index))
            .map(|(_, entry)| entry)
            .collect();
        (filled, unused)
    }

    /// The first context entry with `key`: its position and its value.
    fn context_entry(&self, key: &str) -> Option<(usize, &str)> {
        self.context
            .iter()
            .enumerate()
            .find_map(|(index, (k, value))| (k == key).then_some((index, value.as_str())))
    }
}

// Setters. They take and return the error boxed, like `rustic_core`'s, so
// they chain on the result of `PaceError::new`.
impl PaceError {
    /// Add a line after the guidance message.
    #[must_use]
    pub fn append_guidance_line(self: Box<Self>, line: impl Into<String>) -> Box<Self> {
        let guidance = format!("{}\n{}", self.guidance, line.into());
        Box::new(Self { guidance, ..*self })
    }

    /// Add a line before the guidance message.
    #[must_use]
    pub fn prepend_guidance_line(self: Box<Self>, line: impl Into<String>) -> Box<Self> {
        let guidance = format!("{}\n{}", line.into(), self.guidance);
        Box::new(Self { guidance, ..*self })
    }

    /// Say how to fix the error.
    #[must_use]
    pub fn attach_hint(self: Box<Self>, hint: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            hint: Some(hint.into()),
            ..*self
        })
    }

    /// Attach a context value. A `{key}` placeholder in the guidance is
    /// replaced by it; otherwise it is shown as a detail.
    #[must_use]
    pub fn attach_context(
        mut self: Box<Self>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Box<Self> {
        self.context.push((key.into(), value.into()));
        self
    }

    /// Attach the cause of the error.
    #[must_use]
    pub fn attach_source(
        self: Box<Self>,
        source: impl Into<Box<dyn Error + Send + Sync>>,
    ) -> Box<Self> {
        Box::new(Self {
            source: Some(source.into()),
            ..*self
        })
    }

    /// Attach a stable code, for example `pace::config::unknown-key`.
    #[must_use]
    pub fn attach_error_code(self: Box<Self>, code: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            error_code: Some(code.into()),
            ..*self
        })
    }

    /// Ask the user to report the error; for bugs.
    #[must_use]
    pub fn ask_report(self: Box<Self>) -> Box<Self> {
        Box::new(Self {
            ask_report: true,
            ..*self
        })
    }

    /// Attach how serious the error is.
    #[must_use]
    pub fn attach_severity(self: Box<Self>, severity: Severity) -> Box<Self> {
        Box::new(Self {
            severity: Some(severity),
            ..*self
        })
    }

    /// Set the exit code of the process if this error ends it.
    #[must_use]
    pub fn attach_exit_code(self: Box<Self>, exit_code: ExitCode) -> Box<Self> {
        Box::new(Self { exit_code, ..*self })
    }

    /// Attach the part of a file the error is about.
    #[must_use]
    pub fn attach_snippet(self: Box<Self>, snippet: Snippet) -> Box<Self> {
        Box::new(Self {
            snippet: Some(snippet),
            ..*self
        })
    }
}

impl Display for PaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.filled_guidance().0)
    }
}

impl Error for PaceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| -> &(dyn Error + 'static) { source })
    }
}

impl Diagnostic for PaceError {
    fn code<'error>(&'error self) -> Option<Box<dyn Display + 'error>> {
        self.error_code
            .as_ref()
            .map(|code| -> Box<dyn Display + 'error> { Box::new(code) })
    }

    fn severity(&self) -> Option<miette::Severity> {
        self.severity.map(Into::into)
    }

    fn help<'error>(&'error self) -> Option<Box<dyn Display + 'error>> {
        let mut sections = Vec::new();

        if let Some(hint) = &self.hint {
            sections.push(hint.clone());
        }

        let (_, details) = self.filled_guidance();
        if !details.is_empty() {
            let lines: Vec<_> = details
                .iter()
                .map(|(key, value)| format!("- {key}: {value}"))
                .collect();
            sections.push(format!("Details:\n{}", lines.join("\n")));
        }

        if self.ask_report {
            sections.push(format!(
                "This is a bug in pace. Please report it at {ISSUE_URL}, \
                 with the command you ran."
            ));
        }

        (!sections.is_empty())
            .then(|| -> Box<dyn Display + 'error> { Box::new(sections.join("\n\n")) })
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        self.snippet
            .as_ref()
            .map(|snippet| -> &dyn SourceCode { &snippet.source })
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        self.snippet
            .as_ref()
            .map(|snippet| -> Box<dyn Iterator<Item = LabeledSpan>> {
                Box::new(std::iter::once(LabeledSpan::new_with_span(
                    Some(snippet.label.clone()),
                    snippet.span.clone(),
                )))
            })
    }
}
