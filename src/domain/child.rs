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
    /// Reads the first version-shaped token anywhere in the child's answer.
    ///
    /// Matching the whole line would mean tracking a spelling the child owns
    /// and may reword without notice — and did, which is how a current child
    /// came to be reported as below the floor. Taking the first dotted
    /// three-integer run and ignoring everything around it is the smallest
    /// fact the floor needs (ADR-0095).
    pub(crate) fn parse(bytes: &[u8]) -> Option<Self> {
        std::str::from_utf8(bytes)
            .ok()?
            .split(|character: char| !character.is_ascii_digit() && character != '.')
            .find_map(Self::from_run)
    }
    /// Reads one run of digits and dots, or rejects it as too short.
    ///
    /// Components past the third are discarded rather than ordered, which is
    /// also what happens to a prerelease suffix: the separator ends the run
    /// before the suffix begins.
    fn from_run(run: &str) -> Option<Self> {
        let mut parts = run.split('.').filter(|part| !part.is_empty());
        Some(Self {
            major: parts.next()?.parse().ok()?,
            minor: parts.next()?.parse().ok()?,
            patch: parts.next()?.parse().ok()?,
        })
    }
}

impl std::fmt::Display for ChildVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A fully resolved child invocation.
///
/// `Debug` is hand-written rather than derived, because the environment vector
/// carries whatever ambient `ANTHROPIC_API_KEY` the user has and, for a
/// token-mode launch, the wrapper's own injected token. A derive here would put
/// a credential into any record that ever formatted an invocation, which is the
/// leak the output rules forbid at every level and in every field.
#[derive(Clone, Eq, PartialEq)]
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

impl std::fmt::Debug for ChildInvocation {
    /// Names the program and the environment's keys, and no value.
    ///
    /// Argument bytes are counted rather than printed for the same reason: a
    /// passthrough suffix is the user's, and the wrapper does not decide that
    /// none of it is sensitive.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChildInvocation")
            .field("program", &self.program)
            .field("arguments", &self.arguments.len())
            .field(
                "environment_keys",
                &self
                    .environment
                    .iter()
                    .map(|(key, _)| key)
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// Captured subroutine output and status.
///
/// `Debug` prints lengths rather than bytes: captured output is helper output,
/// which the output rules place alongside credentials as something never
/// emitted at any level or in any field.
pub(crate) struct CapturedChild {
    /// Raw standard output bytes.
    pub(crate) stdout: Vec<u8>,
    /// Raw standard error bytes.
    pub(crate) stderr: Vec<u8>,
    /// The subroutine's status.
    pub(crate) outcome: ChildOutcome,
}

impl std::fmt::Debug for CapturedChild {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedChild")
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr.len())
            .field("outcome", &self.outcome)
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// The shapes a real child has actually printed, plus the ones a reworded
    /// version line could reach. The suffixed form is the one that used to fail
    /// and be reported as below the floor.
    #[test]
    fn a_version_shaped_token_is_read_wherever_it_appears() {
        for bytes in [
            b"2.1.211".as_slice(),
            b"2.1.211\n",
            b"claude 2.1.211\n",
            b"2.1.211 (Claude Code)\n",
            b"v2.1.211",
            b"Claude Code 2.1.211 (build 4073f59)\n",
            // Components past the third are discarded rather than ordered, and
            // a prerelease separator ends the run before its suffix.
            b"2.1.211.4",
            b"2.1.211-rc.1",
        ] {
            assert_eq!(
                ChildVersion::parse(bytes),
                Some(MINIMUM_CHILD_VERSION),
                "{}",
                String::from_utf8_lossy(bytes)
            );
        }
    }

    #[test]
    fn versions_order_by_component_rather_than_by_text() {
        assert!(ChildVersion::parse(b"2.1.210").expect("version") < MINIMUM_CHILD_VERSION);
        assert!(ChildVersion::parse(b"2.1.212").expect("version") > MINIMUM_CHILD_VERSION);
        // The floor's own defect: 220 sorts after 211 by number and before it
        // by text, so the current child must compare as newer.
        assert!(
            ChildVersion::parse(b"2.1.220 (Claude Code)").expect("version") > MINIMUM_CHILD_VERSION
        );
    }

    #[test]
    fn output_with_no_version_shaped_token_is_rejected() {
        for bytes in [
            b"".as_slice(),
            b"2.1",
            b"2.one.3",
            b"unknown",
            b"4294967296.1.1",
            &[0x80],
        ] {
            assert!(
                ChildVersion::parse(bytes).is_none(),
                "{}",
                String::from_utf8_lossy(bytes)
            );
        }
    }
}
