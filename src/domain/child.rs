//! Child invocation and observable outcome values.

use std::{ffi::OsString, path::PathBuf};

/// A fully resolved child invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChildInvocation {
    program: PathBuf,
    arguments: Vec<OsString>,
    environment: Vec<(OsString, OsString)>,
}

impl ChildInvocation {
    /// Builds an invocation without changing OS-string arguments.
    pub(crate) const fn new(
        program: PathBuf,
        arguments: Vec<OsString>,
        environment: Vec<(OsString, OsString)>,
    ) -> Self {
        Self {
            program,
            arguments,
            environment,
        }
    }
    /// Returns the absolute program path.
    pub(crate) const fn program(&self) -> &PathBuf {
        &self.program
    }
    /// Returns the raw child arguments excluding `argv[0]`.
    pub(crate) fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
    /// Returns the exact child environment.
    pub(crate) fn environment(&self) -> &[(OsString, OsString)] {
        &self.environment
    }
}

/// The status observed after waiting for the child.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChildOutcome {
    /// The child exited normally with this code.
    Exited(u8),
    /// The child died from this signal.
    Signaled(i32),
}

/// Captured subroutine output and status.
#[derive(Debug)]
pub(crate) struct CapturedChild {
    /// Raw standard output bytes.
    pub(crate) stdout: Vec<u8>,
    /// Raw standard error bytes.
    pub(crate) stderr: Vec<u8>,
    /// The subroutine's status.
    pub(crate) outcome: ChildOutcome,
}
