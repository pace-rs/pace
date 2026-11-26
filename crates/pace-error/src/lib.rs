//! Errors with fix hints and exit codes for pace.
//!
//! The design follows [`rustic_core`'s error module]: one opaque, boxed
//! [`PaceError`] with a coarse [`ErrorKind`], a guidance message with
//! `{placeholders}` filled from attached context, and builder methods to add
//! more on the way up. pace adds three things: a [hint](PaceError::attach_hint)
//! that says how to fix the error, an [exit code](ExitCode) for scripts, and an
//! optional [source snippet](Snippet) that points at the offending line of a
//! file.
//!
//! [`rustic_core`'s error module]: https://docs.rs/rustic_core/latest/rustic_core/error/index.html
//!
//! # Rules
//!
//! ## Visibility
//!
//! Every fallible `pub fn` returns a [`PaceResult<T>`]. Those errors cross the
//! crate boundary and reach the user, so they carry everything needed: a
//! message, a hint and context.
//!
//! A `pub(crate) fn` uses a local error type instead, for example
//! `pub(crate) type ParseResult<T> = Result<T, ParseErrorKind>` with a
//! `thiserror` enum. At the `pub` boundary, the local error is converted
//! into a [`PaceError`] by hand, with a good message and a hint.
//!
//! ## Upgrading, not downgrading
//!
//! A [`PaceError`] is never turned back into a local error kind. If a
//! function meets a [`PaceError`], its signature is upgraded to return a
//! [`PaceResult`] (or a nested `PaceResult<Result<T, LocalErrorKind>>`).
//!
//! ## Adding to an error instead of wrapping it
//!
//! A [`PaceError`] passed upwards is not wrapped in a new one. Context is
//! attached to it instead: `map_err(|e| e.attach_context("file", path))`,
//! `map_err(|e| e.overwrite_kind(ErrorKind::Project))` or
//! `map_err(|e| e.append_guidance_line("…"))`.
//!
//! ## Hints
//!
//! Errors say how to fix them. Every error a user can cause gets a hint,
//! preferably with the command that fixes it ("`pace begin` starts an
//! entry"). Bugs [ask to be reported](PaceError::ask_report) instead.
//!
//! ## Display
//!
//! Everything attached to an error implements `Display` and `Debug`.
//! `Display` of a [`PaceError`] is the filled guidance only; the
//! [`miette::Diagnostic`] implementation adds the hint, the details, the
//! snippet and the cause chain when the application renders it.

use std::{
    backtrace::Backtrace,
    error::Error,
    fmt::{self, Display},
    ops::Range,
};

use miette::{Diagnostic, LabeledSpan, NamedSource, SourceCode};

mod constants {
    pub const DEFAULT_ISSUE_URL: &str = "https://github.com/pace-rs/pace/issues/new";
}

/// Result type of every fallible public function in pace.
pub type PaceResult<T, E = Box<PaceError>> = Result<T, E>;

/// What an error is related to.
///
/// The `Display` output completes the sentence "an error related to …".
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

impl Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Configuration => "the configuration",
            Self::Directories => "pace's directories",
            Self::Project => "the project context",
            Self::InvalidInput => "invalid user input",
            Self::InputOutput => "input/output operations",
            Self::Time => "time handling",
            Self::Internal => "internal operations",
            Self::Other => "general operations",
        })
    }
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

/// The exit code of the `pace` process, a contract with scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// 0: the command did what was asked.
    Done,
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
            ExitCode::Done => 0,
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

/// An error of pace. See the [crate documentation](crate) for the rules.
#[derive(Debug)]
#[non_exhaustive]
pub struct PaceError {
    /// What the error is related to.
    kind: ErrorKind,

    /// The message, with `{placeholders}` for context values.
    guidance: String,

    /// How to fix the error.
    hint: Option<String>,

    /// The URL of the documentation for the error.
    docs_url: Option<String>,

    /// A stable code for the error.
    error_code: Option<String>,

    /// Whether to ask the user to report the error.
    ask_report: bool,

    /// URLs of already existing issues related to this error.
    existing_issue_urls: Vec<String>,

    /// The URL to open a new issue at.
    new_issue_url: Option<String>,

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

    /// Where the error was created. Captured only if `RUST_BACKTRACE` is set.
    backtrace: Backtrace,
}

impl PaceError {
    /// Create an error of the given kind with a guidance message.
    pub fn new(kind: ErrorKind, guidance: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            kind,
            guidance: guidance.into(),
            hint: None,
            docs_url: None,
            error_code: None,
            ask_report: false,
            existing_issue_urls: Vec::new(),
            new_issue_url: None,
            context: Vec::new(),
            source: None,
            severity: None,
            exit_code: ExitCode::Error,
            snippet: None,
            backtrace: Backtrace::capture(),
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

    /// Create an error from another error, using its message as guidance.
    pub fn from<T: Error + Send + Sync + 'static>(kind: ErrorKind, error: T) -> Box<Self> {
        Self::with_source(kind, error.to_string(), error)
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

    /// Where the error was created, if `RUST_BACKTRACE` was set.
    #[must_use]
    pub const fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    /// The guidance with its placeholders filled, and the context entries
    /// that were not used for a placeholder.
    fn filled_guidance(&self) -> (String, Vec<&(String, String)>) {
        let mut guidance = self.guidance.clone();
        let unused = self
            .context
            .iter()
            .filter(|(key, value)| {
                let pattern = format!("{{{key}}}");
                if guidance.contains(&pattern) {
                    guidance = guidance.replace(&pattern, value);
                    false
                } else {
                    true
                }
            })
            .collect();
        (guidance, unused)
    }

    /// A one-line form for logs.
    #[must_use]
    pub fn display_log(&self) -> String {
        let (guidance, _) = self.filled_guidance();
        let mut log = format!("Error: {guidance} (kind: related to {}", self.kind);
        if let Some(code) = &self.error_code {
            log.push_str(", code: ");
            log.push_str(code);
        }
        log.push(')');
        if let Some(cause) = &self.source {
            log.push_str(": caused by: ");
            log.push_str(&cause.to_string());
            if let Some(source) = cause.source() {
                log.push_str(" : (source: ");
                log.push_str(&source.to_string());
                log.push(')');
            }
        }
        log
    }
}

// Setters. They take and return the error boxed, like `rustic_core`'s, so
// they chain on the result of `PaceError::new`.
impl PaceError {
    /// Change what the error is related to.
    #[must_use]
    pub fn overwrite_kind(self: Box<Self>, kind: ErrorKind) -> Box<Self> {
        Box::new(Self { kind, ..*self })
    }

    /// Replace the guidance message.
    #[must_use]
    pub fn overwrite_guidance(self: Box<Self>, guidance: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            guidance: guidance.into(),
            ..*self
        })
    }

    /// Add a line after the guidance message.
    #[must_use]
    pub fn append_guidance_line(self: Box<Self>, line: impl Into<String>) -> Box<Self> {
        let guidance = format!("{}\n{}", self.guidance, line.into());
        self.overwrite_guidance(guidance)
    }

    /// Add a line before the guidance message.
    #[must_use]
    pub fn prepend_guidance_line(self: Box<Self>, line: impl Into<String>) -> Box<Self> {
        let guidance = format!("{}\n{}", line.into(), self.guidance);
        self.overwrite_guidance(guidance)
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

    /// Attach the URL of the documentation for the error.
    #[must_use]
    pub fn attach_docs_url(self: Box<Self>, url: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            docs_url: Some(url.into()),
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

    /// Attach the URL to open a new issue at, instead of the default.
    #[must_use]
    pub fn attach_new_issue_url(self: Box<Self>, url: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            new_issue_url: Some(url.into()),
            ..*self
        })
    }

    /// Attach the URL of an existing issue related to this error.
    #[must_use]
    pub fn attach_existing_issue_url(mut self: Box<Self>, url: impl Into<String>) -> Box<Self> {
        self.existing_issue_urls.push(url.into());
        self
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
    fn code<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.error_code
            .as_ref()
            .map(|code| -> Box<dyn Display + 'a> { Box::new(code) })
    }

    fn severity(&self) -> Option<miette::Severity> {
        self.severity.map(Into::into)
    }

    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
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

        if !self.existing_issue_urls.is_empty() {
            let lines: Vec<_> = self
                .existing_issue_urls
                .iter()
                .map(|url| format!("- {url}"))
                .collect();
            sections.push(format!("Related issues:\n{}", lines.join("\n")));
        }

        if self.ask_report {
            let url = self
                .new_issue_url
                .as_deref()
                .unwrap_or(constants::DEFAULT_ISSUE_URL);
            sections.push(format!(
                "This is a bug in pace. Please report it at {url}, \
                 with the command you ran."
            ));
        }

        (!sections.is_empty()).then(|| -> Box<dyn Display + 'a> { Box::new(sections.join("\n\n")) })
    }

    fn url<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.docs_url
            .as_ref()
            .map(|url| -> Box<dyn Display + 'a> { Box::new(url) })
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
