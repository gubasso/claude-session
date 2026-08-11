//! Child invocation and observable outcome values.

use std::{ffi::OsString, path::PathBuf};

/// The minimum child release whose refresh-lock behaviour is supported.
pub(crate) const MINIMUM_CHILD_VERSION: ChildVersion = ChildVersion {
    major: 2,
    minor: 1,
    patch: 211,
};

/// A validated three-component child version.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ChildVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl ChildVersion {
    /// Parses either `claude X.Y.Z` or a bare `X.Y.Z` answer.
    pub(crate) fn parse(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?.trim();
        let token = text.strip_prefix("claude ").unwrap_or(text);
        let mut parts = token.split('.');
        let value = Self {
            major: parts.next()?.parse().ok()?,
            minor: parts.next()?.parse().ok()?,
            patch: parts.next()?.parse().ok()?,
        };
        (parts.next().is_none()).then_some(value)
    }
}

impl std::fmt::Display for ChildVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn child_versions_accept_the_documented_shapes_and_order() {
        assert_eq!(
            ChildVersion::parse(b"claude 2.1.211\n"),
            Some(MINIMUM_CHILD_VERSION)
        );
        assert_eq!(ChildVersion::parse(b"2.1.211"), Some(MINIMUM_CHILD_VERSION));
        assert!(ChildVersion::parse(b"2.1.210").expect("version") < MINIMUM_CHILD_VERSION);
        assert!(ChildVersion::parse(b"2.1.212").expect("version") > MINIMUM_CHILD_VERSION);
    }

    #[test]
    fn malformed_child_versions_are_rejected() {
        for bytes in [
            b"".as_slice(),
            b"2.1",
            b"2.one.3",
            b"2.1.3.4",
            b"4294967296.1.1",
            &[0x80],
        ] {
            assert!(ChildVersion::parse(bytes).is_none());
        }
    }
}
