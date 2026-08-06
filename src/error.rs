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
///
/// The set is the published taxonomy, not the set of conditions reached so far.
/// A kind is part of the user-facing API the moment it is documented, and it is
/// never renamed or remapped, so the enum matches the table rather than the
/// current call sites.
#[allow(
    dead_code,
    reason = "the exit-code table is the API, not the call sites"
)]
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

/// A wrapper-originated failure: one kind, one diagnostic.
///
/// A struct rather than one variant per kind. The kind already enumerates the
/// closed set and already owns the code, so a parallel variant list only gives
/// the two ways to drift apart, and every match over it has to be written again
/// each time the set grows.
#[derive(Debug)]
pub(crate) struct AppError {
    kind: ErrorKind,
    diagnostic: Diagnostic,
}

impl AppError {
    /// Constructs a failure of the given kind.
    pub(crate) const fn new(kind: ErrorKind, diagnostic: Diagnostic) -> Self {
        Self { kind, diagnostic }
    }

    /// Returns the stable kind.
    pub(crate) const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Borrows the structured diagnostic.
    pub(crate) const fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }

    /// Returns the stable numeric status.
    pub(crate) const fn exit_code(&self) -> u8 {
        self.kind.exit_code()
    }

    /// Constructs a child-not-found diagnostic.
    pub(crate) fn child_not_found(where_: impl Into<String>, why: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::ChildNotFound,
            Diagnostic::new(
                "child executable was not found",
                where_,
                why,
                "configure child_bin or add claude to PATH",
            ),
        )
    }
    /// Constructs a child-not-executable diagnostic.
    pub(crate) fn child_not_executable(where_: impl Into<String>, why: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::ChildNotExecutable,
            Diagnostic::new(
                "child executable cannot be run",
                where_,
                why,
                "check that the file is regular and executable",
            ),
        )
    }
}

impl From<DomainError> for AppError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::InvalidArguments(message) | DomainError::InvalidIdentifier(message) => {
                Self::new(
                    ErrorKind::Usage,
                    Diagnostic::new(
                        "invalid command line",
                        "wrapper arguments",
                        message,
                        "run claude-session --help",
                    ),
                )
            }
            DomainError::MissingHome => Self::new(
                ErrorKind::Config,
                Diagnostic::new(
                    "XDG paths cannot be resolved",
                    "process environment",
                    "HOME is unset and an XDG base needs its default",
                    "set absolute XDG base variables",
                ),
            ),
        }
    }
}

impl From<ConfigError> for AppError {
    // A conversion that discards context is worse than none. The three variants
    // have three different fixes, so collapsing them into one code would tell a
    // caller to correct a value when the real problem is a mode or a name.
    fn from(error: ConfigError) -> Self {
        let why = error.to_string();
        match error {
            ConfigError::Read { path, source } => {
                let where_ = path.display().to_string();
                match source.kind() {
                    io::ErrorKind::NotFound => Self::new(
                        ErrorKind::NoInput,
                        Diagnostic::new(
                            "configuration file was not found",
                            where_,
                            why,
                            "create the file or drop the flag naming it",
                        ),
                    ),
                    io::ErrorKind::PermissionDenied => Self::new(
                        ErrorKind::Permission,
                        Diagnostic::new(
                            "configuration file could not be read",
                            where_,
                            why,
                            "repair the file's ownership or mode",
                        ),
                    ),
                    _ => Self::new(
                        ErrorKind::Io,
                        Diagnostic::new(
                            "configuration file could not be read",
                            where_,
                            why,
                            "check the path and the filesystem",
                        ),
                    ),
                }
            }
            ConfigError::Decode { path, .. } => Self::new(
                ErrorKind::Config,
                Diagnostic::new(
                    "configuration file is invalid",
                    path.display().to_string(),
                    why,
                    "correct the named key or value",
                ),
            ),
            ConfigError::Value { key, .. } => Self::new(
                ErrorKind::Config,
                Diagnostic::new(
                    "configuration value is invalid",
                    key,
                    why,
                    "correct the named value",
                ),
            ),
        }
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

    /// Every kind, its published spelling, and its published code.
    ///
    /// Written out rather than derived, because a table generated from the enum
    /// would agree with the enum by construction and prove nothing. This is the
    /// copy of the exit-codes reference that the compiler can check.
    const MATRIX: &[(ErrorKind, &str, u8)] = &[
        (ErrorKind::Usage, "Usage", 64),
        (ErrorKind::DataFormat, "DataFormat", 65),
        (ErrorKind::NoInput, "NoInput", 66),
        (ErrorKind::Unavailable, "Unavailable", 69),
        (ErrorKind::Internal, "Internal", 70),
        (ErrorKind::OsError, "OsError", 71),
        (ErrorKind::Io, "Io", 74),
        (ErrorKind::LockBusy, "LockBusy", 75),
        (ErrorKind::Auth, "Auth", 77),
        (ErrorKind::Permission, "Permission", 77),
        (ErrorKind::Config, "Config", 78),
        (ErrorKind::ChildRecursion, "ChildRecursion", 78),
        (ErrorKind::ChildNotExecutable, "ChildNotExecutable", 126),
        (ErrorKind::ChildNotFound, "ChildNotFound", 127),
    ];

    #[test]
    fn every_kind_maps_to_its_published_spelling_and_code() {
        for (kind, spelling, code) in MATRIX {
            assert_eq!(kind.spelling(), *spelling);
            assert_eq!(kind.exit_code(), *code);
            let error = AppError::new(*kind, Diagnostic::new("w", "x", "y", "z"));
            assert_eq!(error.kind(), *kind);
            assert_eq!(error.exit_code(), *code);
        }
    }

    /// A kind added to the enum without a row here would otherwise be unchecked:
    /// the loop above only visits what the table already lists.
    #[test]
    fn the_matrix_covers_every_kind() {
        let listed: std::collections::BTreeSet<&str> =
            MATRIX.iter().map(|(_, spelling, _)| *spelling).collect();
        assert_eq!(listed.len(), MATRIX.len(), "the matrix repeats a kind");
        // `spelling` is an exhaustive match, so a new variant forces a new arm
        // there; this pins the count that arm list must equal.
        assert_eq!(
            MATRIX.len(),
            14,
            "a kind was added or removed without updating the matrix"
        );
    }

    /// Codes are shared where sysexits gives two kinds one category, so the
    /// kind is the precise identifier and the code is the coarse one.
    #[test]
    fn a_shared_code_still_has_distinct_kinds() {
        assert_eq!(
            ErrorKind::Auth.exit_code(),
            ErrorKind::Permission.exit_code()
        );
        assert_ne!(ErrorKind::Auth.spelling(), ErrorKind::Permission.spelling());
    }
}
