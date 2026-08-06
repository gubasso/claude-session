//! Closed, typed application errors and stable diagnostics.

use std::{fmt, io, path::PathBuf};

/// Pure domain validation failures.
#[derive(Debug, thiserror::Error)]
pub(crate) enum DomainError {
    /// An identifier violated the documented grammar.
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
    /// Wrapper arguments were malformed.
    #[error("invalid wrapper arguments: {0}")]
    InvalidArguments(String),
    /// No home directory was available for XDG defaults.
    #[error("HOME is unavailable")]
    MissingHome,
}

/// Configuration loading failures.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    /// A file could not be read.
    #[error("cannot read configuration file {path}: {source}")]
    Read { path: PathBuf, source: io::Error },
    /// A file could not be decoded strictly.
    #[error("invalid configuration file {path}: {message}")]
    Decode { path: PathBuf, message: String },
    /// An environment or file identifier was invalid.
    #[error("invalid configuration value for {key} from {origin}")]
    Value { key: &'static str, origin: String },
}

/// Stable wrapper error identifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ErrorKind {
    /// Command-line usage failed.
    Usage,
    /// Structured input was malformed.
    DataFormat,
    /// A required input was absent.
    NoInput,
    /// A required service was unavailable.
    Unavailable,
    /// An internal invariant failed.
    Internal,
    /// An operating-system operation failed.
    OsError,
    /// Generic input/output failed.
    Io,
    /// A lock was busy.
    LockBusy,
    /// Authentication failed.
    Auth,
    /// Permission was denied.
    Permission,
    /// Wrapper configuration was invalid.
    Config,
    /// Child recursion was detected.
    ChildRecursion,
    /// The child exists but cannot execute.
    ChildNotExecutable,
    /// The child could not be found.
    ChildNotFound,
}

impl ErrorKind {
    /// Returns the append-only public spelling.
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::Usage => "Usage",
            Self::DataFormat => "DataFormat",
            Self::NoInput => "NoInput",
            Self::Unavailable => "Unavailable",
            Self::Internal => "Internal",
            Self::OsError => "OsError",
            Self::Io => "Io",
            Self::LockBusy => "LockBusy",
            Self::Auth => "Auth",
            Self::Permission => "Permission",
            Self::Config => "Config",
            Self::ChildRecursion => "ChildRecursion",
            Self::ChildNotExecutable => "ChildNotExecutable",
            Self::ChildNotFound => "ChildNotFound",
        }
    }

    /// Returns the stable wrapper-owned process code.
    pub(crate) const fn exit_code(self) -> u8 {
        match self {
            Self::Usage => 64,
            Self::DataFormat => 65,
            Self::NoInput => 66,
            Self::Unavailable => 69,
            Self::Internal => 70,
            Self::OsError => 71,
            Self::Io => 74,
            Self::LockBusy => 75,
            Self::Auth | Self::Permission => 77,
            Self::Config | Self::ChildRecursion => 78,
            Self::ChildNotExecutable => 126,
            Self::ChildNotFound => 127,
        }
    }
}

/// The four-part human diagnostic plus optional child status.
#[derive(Clone, Debug)]
pub(crate) struct Diagnostic {
    /// Short statement of the failure.
    pub(crate) what: String,
    /// Concrete location or subject.
    pub(crate) where_: String,
    /// Explanation of the cause.
    pub(crate) why: String,
    /// One useful recovery action.
    pub(crate) hint: String,
    /// Child status when relevant.
    pub(crate) child_exit: Option<u8>,
}

impl Diagnostic {
    /// Constructs a diagnostic with no child status.
    pub(crate) fn new(
        what: impl Into<String>,
        where_: impl Into<String>,
        why: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self {
            what: what.into(),
            where_: where_.into(),
            why: why.into(),
            hint: hint.into(),
            child_exit: None,
        }
    }
}

/// The closed set of wrapper-originated failures.
#[allow(dead_code)] // ADR-0005/ADR-0035 require the future-proof exhaustive matrix.
#[derive(Debug)]
pub(crate) enum AppError {
    /// Usage failure.
    Usage(Diagnostic),
    DataFormat(Diagnostic),
    NoInput(Diagnostic),
    Unavailable(Diagnostic),
    Internal(Diagnostic),
    OsError(Diagnostic),
    Io(Diagnostic),
    LockBusy(Diagnostic),
    Auth(Diagnostic),
    Permission(Diagnostic),
    Config(Diagnostic),
    ChildRecursion(Diagnostic),
    ChildNotExecutable(Diagnostic),
    ChildNotFound(Diagnostic),
}

impl AppError {
    /// Returns the stable kind without a catch-all arm.
    pub(crate) const fn kind(&self) -> ErrorKind {
        match self {
            Self::Usage(_) => ErrorKind::Usage,
            Self::DataFormat(_) => ErrorKind::DataFormat,
            Self::NoInput(_) => ErrorKind::NoInput,
            Self::Unavailable(_) => ErrorKind::Unavailable,
            Self::Internal(_) => ErrorKind::Internal,
            Self::OsError(_) => ErrorKind::OsError,
            Self::Io(_) => ErrorKind::Io,
            Self::LockBusy(_) => ErrorKind::LockBusy,
            Self::Auth(_) => ErrorKind::Auth,
            Self::Permission(_) => ErrorKind::Permission,
            Self::Config(_) => ErrorKind::Config,
            Self::ChildRecursion(_) => ErrorKind::ChildRecursion,
            Self::ChildNotExecutable(_) => ErrorKind::ChildNotExecutable,
            Self::ChildNotFound(_) => ErrorKind::ChildNotFound,
        }
    }

    /// Borrows the structured diagnostic.
    pub(crate) const fn diagnostic(&self) -> &Diagnostic {
        match self {
            Self::Usage(value)
            | Self::DataFormat(value)
            | Self::NoInput(value)
            | Self::Unavailable(value)
            | Self::Internal(value)
            | Self::OsError(value)
            | Self::Io(value)
            | Self::LockBusy(value)
            | Self::Auth(value)
            | Self::Permission(value)
            | Self::Config(value)
            | Self::ChildRecursion(value)
            | Self::ChildNotExecutable(value)
            | Self::ChildNotFound(value) => value,
        }
    }

    /// Returns the stable numeric status.
    pub(crate) const fn exit_code(&self) -> u8 {
        self.kind().exit_code()
    }
    /// Constructs a child-not-found diagnostic.
    pub(crate) fn child_not_found(where_: impl Into<String>, why: impl Into<String>) -> Self {
        Self::ChildNotFound(Diagnostic::new(
            "child executable was not found",
            where_,
            why,
            "configure child_bin or add claude to PATH",
        ))
    }
    /// Constructs a child-not-executable diagnostic.
    pub(crate) fn child_not_executable(where_: impl Into<String>, why: impl Into<String>) -> Self {
        Self::ChildNotExecutable(Diagnostic::new(
            "child executable cannot be run",
            where_,
            why,
            "check that the file is regular and executable",
        ))
    }
}

impl From<DomainError> for AppError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::InvalidArguments(message) | DomainError::InvalidIdentifier(message) => {
                Self::Usage(Diagnostic::new(
                    "invalid command line",
                    "wrapper arguments",
                    message,
                    "run claude-session --help",
                ))
            }
            DomainError::MissingHome => Self::Config(Diagnostic::new(
                "XDG paths cannot be resolved",
                "process environment",
                "HOME is unset and an XDG base needs its default",
                "set absolute XDG base variables",
            )),
        }
    }
}

impl From<ConfigError> for AppError {
    fn from(error: ConfigError) -> Self {
        Self::Config(Diagnostic::new(
            "configuration is invalid",
            "configuration loading",
            error.to_string(),
            "correct the named value or file",
        ))
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let diagnostic = self.diagnostic();
        write!(
            formatter,
            "claude-session: error[{}]: {}\nWhere: {}\nWhy: {}\nHint: {}",
            self.kind().spelling(),
            diagnostic.what,
            diagnostic.where_,
            diagnostic.why,
            diagnostic.hint
        )?;
        if let Some(code) = diagnostic.child_exit {
            write!(formatter, "\nChild exit: {code}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exit_code_matrix_is_exhaustive() {
        let diagnostic = || Diagnostic::new("w", "x", "y", "z");
        let cases = [
            (AppError::Usage(diagnostic()), "Usage", 64),
            (AppError::DataFormat(diagnostic()), "DataFormat", 65),
            (AppError::NoInput(diagnostic()), "NoInput", 66),
            (AppError::Unavailable(diagnostic()), "Unavailable", 69),
            (AppError::Internal(diagnostic()), "Internal", 70),
            (AppError::OsError(diagnostic()), "OsError", 71),
            (AppError::Io(diagnostic()), "Io", 74),
            (AppError::LockBusy(diagnostic()), "LockBusy", 75),
            (AppError::Auth(diagnostic()), "Auth", 77),
            (AppError::Permission(diagnostic()), "Permission", 77),
            (AppError::Config(diagnostic()), "Config", 78),
            (AppError::ChildRecursion(diagnostic()), "ChildRecursion", 78),
            (
                AppError::ChildNotExecutable(diagnostic()),
                "ChildNotExecutable",
                126,
            ),
            (AppError::ChildNotFound(diagnostic()), "ChildNotFound", 127),
        ];
        for (error, spelling, code) in cases {
            assert_eq!(error.kind().spelling(), spelling);
            assert_eq!(error.exit_code(), code);
        }
    }
}
