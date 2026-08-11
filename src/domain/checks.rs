//! The complete public probe catalog and report values.
//!
//! This module is for naming a storage condition and quoting the one wording
//! that answers it, so a guard and a later `doctor` describe one problem in one
//! sentence ([ADR-0018]). It is not for running the checks, which is
//! `services::storage::guard`, and not for rendering a report, which the
//! `doctor` verb will own.
//!
//! [ADR-0018]: ../../docs/decisions/ADR-0018-one-probe-set-with-stable-check-ids.md

use std::path::Path;

use crate::error::{Diagnostic, ErrorKind};

/// The ownership boundary a check inspects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Scope {
    Host,
    Session,
}

impl Scope {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Session => "session",
        }
    }
}

/// Whether an unhealthy subject prevents the wrapper from functioning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Severity {
    Hard,
    Soft,
}

impl Severity {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Hard => "hard",
            Self::Soft => "soft",
        }
    }
}

/// One stable catalog entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Check {
    BaseDirsResolve,
    RuntimeDirPresent,
    WrapperConfigParses,
    ChildBinaryResolves,
    ChildIsExecutable,
    ChildVersionFloor,
    Storage(StorageCheck),
    Entry(EntryCheck),
    Account(AccountCheck),
}

/// The implemented public catalog, in its append-only order.
pub(crate) const CATALOG: &[Check] = &[
    Check::BaseDirsResolve,
    Check::RuntimeDirPresent,
    Check::WrapperConfigParses,
    Check::ChildBinaryResolves,
    Check::ChildIsExecutable,
    Check::ChildVersionFloor,
    Check::Storage(StorageCheck::NoSymlinks),
    Check::Storage(StorageCheck::Owned),
    Check::Storage(StorageCheck::Typed),
    Check::Storage(StorageCheck::DirectoryModes),
    Check::Storage(StorageCheck::SecretModes),
    Check::Entry(EntryCheck::Compose),
    Check::Entry(EntryCheck::Consistent),
    Check::Account(AccountCheck::RegistryReadable),
    Check::Account(AccountCheck::CredentialsUsable),
];

impl Check {
    /// Returns the stable public id.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::BaseDirsResolve => "base-dirs-resolve",
            Self::RuntimeDirPresent => "runtime-dir-present",
            Self::WrapperConfigParses => "wrapper-config-parses",
            Self::ChildBinaryResolves => "child-binary-resolves",
            Self::ChildIsExecutable => "child-is-executable",
            Self::ChildVersionFloor => "child-version-floor",
            Self::Storage(value) => value.id(),
            Self::Entry(value) => value.id(),
            Self::Account(value) => value.id(),
        }
    }
    /// Returns the owning scope.
    pub(crate) const fn scope(self) -> Scope {
        match self {
            Self::BaseDirsResolve
            | Self::RuntimeDirPresent
            | Self::WrapperConfigParses
            | Self::ChildBinaryResolves
            | Self::ChildIsExecutable
            | Self::ChildVersionFloor => Scope::Host,
            Self::Storage(_) | Self::Entry(_) | Self::Account(_) => Scope::Session,
        }
    }
    /// Returns the published severity.
    pub(crate) const fn severity(self) -> Severity {
        match self {
            Self::RuntimeDirPresent | Self::ChildVersionFloor | Self::Account(_) => Severity::Soft,
            _ => Severity::Hard,
        }
    }
    /// Returns the published error kind.
    pub(crate) const fn kind(self) -> ErrorKind {
        match self {
            Self::BaseDirsResolve | Self::RuntimeDirPresent | Self::ChildVersionFloor => {
                ErrorKind::Unavailable
            }
            Self::WrapperConfigParses => ErrorKind::Config,
            Self::ChildBinaryResolves => ErrorKind::ChildNotFound,
            Self::ChildIsExecutable => ErrorKind::ChildNotExecutable,
            Self::Storage(value) => value.kind(),
            Self::Entry(value) => value.kind(),
            Self::Account(value) => value.kind(),
        }
    }
    /// Returns the remediation template, if this check owns one.
    pub(crate) const fn remediation(self) -> Option<&'static str> {
        match self {
            Self::BaseDirsResolve => Some(concat!(
                "`XDG_CONFIG_HOME` and `XDG_STATE_HOME` must be absolute paths, or unset so ",
                "the defaults apply. Run `claude-session doctor` to see what each resolved to."
            )),
            Self::RuntimeDirPresent => None,
            Self::WrapperConfigParses => Some(concat!(
                "`{key}` in `{path}` is not a configuration key. Remove it, or correct it ",
                "to one of the keys [configuration](./configuration.md#keys) lists."
            )),
            Self::ChildBinaryResolves => Some(concat!(
                "No `claude` was found. Install it, put it on `PATH`, or set `child_bin` ",
                "to its absolute path — [process runtime](./process-runtime.md#child-resolution) ",
                "gives the order the two rungs are tried in."
            )),
            Self::ChildIsExecutable => Some(concat!(
                "{path} exists but the current user cannot execute it. Grant execute permission, ",
                "or point `child_bin` at a different binary."
            )),
            Self::ChildVersionFloor => Some(concat!(
                "The resolved `claude` reports {version}, below the {minimum} this wrapper is ",
                "designed against. Upgrade it before using a saved-login account; token mode ",
                "still works below the floor."
            )),
            Self::Storage(value) => Some(value.remediation()),
            Self::Entry(value) => Some(value.remediation()),
            Self::Account(value) => Some(value.remediation()),
        }
    }
    /// Substitutes named values into this check's owned remediation.
    pub(crate) fn hint(self, values: &[(&str, &str)]) -> Option<String> {
        let mut text = self.remediation()?.to_owned();
        for (name, value) in values {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        Some(text)
    }
}

/// Account-local health checks appended by slice 005.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AccountCheck {
    RegistryReadable,
    CredentialsUsable,
}

impl AccountCheck {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::RegistryReadable => "account-registry-readable",
            Self::CredentialsUsable => "credentials-usable",
        }
    }
    pub(crate) const fn kind(self) -> ErrorKind {
        match self {
            Self::RegistryReadable => ErrorKind::Io,
            Self::CredentialsUsable => ErrorKind::Auth,
        }
    }
    pub(crate) const fn remediation(self) -> &'static str {
        match self {
            Self::RegistryReadable => {
                "Make `{path}` a readable, private directory owned by the current user, then retry."
            }
            Self::CredentialsUsable => {
                "Run `claude-session account login {account}` to recreate this account's stored \
                authentication and its local metadata."
            }
        }
    }
}

/// The outcome of one public check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Skipped,
}
impl CheckStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::Skipped => "skipped",
        }
    }
}

/// One rendered catalog observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CheckResult {
    pub(crate) check: Check,
    pub(crate) status: CheckStatus,
    pub(crate) message: String,
    pub(crate) hint: Option<String>,
    pub(crate) reason: Option<String>,
}
impl CheckResult {
    /// Builds a healthy observation.
    pub(crate) fn pass(check: Check, message: impl Into<String>) -> Self {
        Self {
            check,
            status: CheckStatus::Pass,
            message: message.into(),
            hint: None,
            reason: None,
        }
    }
    /// Builds an unhealthy observation using the check's severity.
    pub(crate) fn defect(check: Check, message: impl Into<String>, hint: String) -> Self {
        Self {
            check,
            status: if check.severity() == Severity::Hard {
                CheckStatus::Fail
            } else {
                CheckStatus::Warn
            },
            message: message.into(),
            // `runtime-dir-present` owns no template, so an empty hint is an
            // absent one rather than a blank line under the row.
            hint: (!hint.is_empty()).then_some(hint),
            reason: None,
        }
    }
    /// Builds a prerequisite or inapplicability skip.
    pub(crate) fn skipped(check: Check, reason: impl Into<String>) -> Self {
        Self {
            check,
            status: CheckStatus::Skipped,
            message: "not checked".into(),
            hint: None,
            reason: Some(reason.into()),
        }
    }
    /// Returns a kind only for unhealthy results.
    pub(crate) const fn kind(&self) -> Option<ErrorKind> {
        match self.status {
            CheckStatus::Warn | CheckStatus::Fail => Some(self.check.kind()),
            _ => None,
        }
    }
}

/// Deterministic public result counts and the wrapper level's own status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Summary {
    pub(crate) total: usize,
    pub(crate) passed: usize,
    pub(crate) warned: usize,
    pub(crate) failed: usize,
    pub(crate) skipped: usize,
    pub(crate) hard_failures: usize,
    pub(crate) exit: u8,
}
impl Summary {
    /// Folds catalog results in input order into the wrapper level alone.
    ///
    /// `--strict` is not applied here: promotion is a verdict-level policy over
    /// both levels, and this count describes only the wrapper's own catalog.
    pub(crate) fn fold(results: &[CheckResult]) -> Self {
        let mut value = Self {
            total: results.len(),
            passed: 0,
            warned: 0,
            failed: 0,
            skipped: 0,
            hard_failures: 0,
            exit: 0,
        };
        for result in results {
            match result.status {
                CheckStatus::Pass => value.passed += 1,
                CheckStatus::Warn => value.warned += 1,
                CheckStatus::Fail => {
                    value.failed += 1;
                    value.hard_failures += 1;
                    if value.exit == 0 {
                        value.exit = result.check.kind().exit_code();
                    }
                }
                CheckStatus::Skipped => value.skipped += 1,
            }
        }
        value
    }
    /// Returns this level's own status word.
    pub(crate) const fn status(&self) -> CheckStatus {
        if self.hard_failures > 0 {
            CheckStatus::Fail
        } else if self.warned > 0 {
            CheckStatus::Warn
        } else {
            CheckStatus::Pass
        }
    }
}

/// How the child's own report ended.
///
/// The child is the application this wrapper exists to run, so its level is
/// carried across unchanged: a failed child report is a failure here too, never
/// a warning ([ADR-0085]).
///
/// [ADR-0085]: ../../docs/decisions/ADR-0085-carry-the-child-report-level-into-the-verdict.md
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChildStatus {
    Pass,
    Fail,
    Skipped,
}
impl ChildStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Skipped => "skipped",
        }
    }
}

/// The child's own report, the second level of a composed run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChildReport {
    pub(crate) status: ChildStatus,
    pub(crate) output: Option<Vec<u8>>,
    pub(crate) exit: Option<u8>,
    pub(crate) reason: Option<String>,
}
impl ChildReport {
    /// Builds the report of a child that never ran.
    pub(crate) fn skipped(reason: impl Into<String>) -> Self {
        Self {
            status: ChildStatus::Skipped,
            output: None,
            exit: None,
            reason: Some(reason.into()),
        }
    }
}

/// The composed status and exit over the wrapper and child levels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Verdict {
    pub(crate) status: CheckStatus,
    pub(crate) exit: u8,
}
impl Verdict {
    /// Folds both levels into the one status the process reports.
    ///
    /// A wrapper hard failure wins the code, because the wrapper's own defect
    /// usually explains the child's. Otherwise a failed child report exits
    /// `Unavailable`: the level is the child's, the code stays in the wrapper's
    /// matrix, and the child's own code is reported as data ([ADR-0085]).
    ///
    /// [ADR-0085]: ../../docs/decisions/ADR-0085-carry-the-child-report-level-into-the-verdict.md
    pub(crate) fn fold(summary: Summary, child: &ChildReport, strict: bool) -> Self {
        let status = if summary.hard_failures > 0 || child.status == ChildStatus::Fail {
            CheckStatus::Fail
        } else if summary.warned > 0 {
            CheckStatus::Warn
        } else {
            CheckStatus::Pass
        };
        let exit = if summary.exit != 0 {
            summary.exit
        } else if child.status == ChildStatus::Fail {
            ErrorKind::Unavailable.exit_code()
        } else {
            u8::from(strict && summary.warned > 0)
        };
        Self { status, exit }
    }
}

/// A storage condition with a stable, public check identifier.
///
/// Five ids rather than one because each condition has a different remedy, and
/// a check owns exactly one remediation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StorageCheck {
    /// No existing wrapper-managed path component is a symbolic link.
    NoSymlinks,
    /// Every existing wrapper-managed path component is owned by this user.
    Owned,
    /// Every wrapper-managed path has the type the artifact table assigns it.
    Typed,
    /// Every wrapper-managed directory is `0700` after automatic correction.
    DirectoryModes,
    /// Every wrapper-owned private file is `0600` after correction.
    SecretModes,
}

impl StorageCheck {
    /// Returns the stable, public check identifier.
    ///
    /// Ids are public API: scripts match them and messages cite them, so
    /// renaming one is a breaking change.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::NoSymlinks => "storage-paths-no-symlinks",
            Self::Owned => "storage-paths-owned",
            Self::Typed => "storage-paths-typed",
            Self::DirectoryModes => "storage-directory-modes",
            Self::SecretModes => "storage-secret-modes",
        }
    }

    /// Returns the kind a failure of this check exits with.
    pub(crate) const fn kind(self) -> ErrorKind {
        match self {
            Self::NoSymlinks
            | Self::Owned
            | Self::Typed
            | Self::DirectoryModes
            | Self::SecretModes => ErrorKind::Permission,
        }
    }

    /// Returns the one remediation template this check owns.
    ///
    /// Copied verbatim from the catalog. A guard quotes the template and
    /// substitutes into it; it never paraphrases, because a user who hits the
    /// guard and a user who runs `doctor` must read one wording.
    ///
    /// The credential clause the catalog appends on `oauth-token`,
    /// `auth-mode.json`, and the child's `.credentials.json` is deliberately
    /// absent: this slice creates none of those three, so a clause with no
    /// producer would discriminate nothing. It belongs with the account
    /// subsystem that first writes one.
    pub(crate) const fn remediation(self) -> &'static str {
        match self {
            Self::NoSymlinks => {
                "Move the symbolic link at {path} aside and recreate the expected \
                {expected_type} there, restoring only content you trust."
            }
            Self::Owned => {
                "{path} is owned by another user, which usually means a restored backup \
                or a file created under sudo. Do not change its owner in place — move \
                it aside and let the wrapper recreate it as you."
            }
            Self::Typed => {
                "{path} is a {actual_type} and this location must be a {expected_type}. \
                Move it aside and let the wrapper recreate it; nothing under this path \
                is unrecoverable except an account login."
            }
            Self::DirectoryModes => {
                "Could not restrict {path} to mode {expected_mode}. Check that it is on \
                a filesystem supporting Unix permissions and was created by the current \
                user."
            }
            Self::SecretModes => {
                "Could not restrict {path} to mode {expected_mode}. Move the file to \
                storage that supports Unix permissions before using it again."
            }
        }
    }

    /// Renders this check's failure as the four-part diagnostic.
    ///
    /// What, Where, and Why are computed from the failure; the Hint is the
    /// template with its placeholders substituted and nothing else changed.
    // The placeholders are the catalog's own spelling, not this crate's format
    // syntax; they are substituted, never formatted.
    #[allow(clippy::literal_string_with_formatting_args)]
    pub(crate) fn diagnostic(
        self,
        path: &Path,
        expected: &str,
        actual: &str,
        mode: Option<u32>,
    ) -> Diagnostic {
        let rendered = path.display().to_string();
        let expected_mode = mode.map_or_else(String::new, |value| format!("{value:04o}"));
        let hint = self
            .remediation()
            .replace("{path}", &rendered)
            .replace("{expected_type}", expected)
            .replace("{actual_type}", actual)
            .replace("{expected_mode}", &expected_mode);
        Diagnostic::new(
            "wrapper-managed storage refused a path",
            rendered,
            format!("{} failed", self.id()),
            hint,
        )
    }
}

/// A composed-entry condition, alongside the five path checks above.
///
/// Separate from [`StorageCheck`] because these two do not share a kind: one is
/// a missing input and the other is an entry that disagrees with itself, and
/// collapsing them would put one id in charge of two unrelated instructions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EntryCheck {
    /// A resolved profile, and every piece it names, exists.
    Compose,
    /// A materialized entry's recorded digest matches the one its inputs recompute.
    Consistent,
}

impl EntryCheck {
    /// Returns the stable, public check identifier.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::Compose => "settings-compose",
            Self::Consistent => "settings-entry-consistent",
        }
    }

    /// Returns the kind a failure of this check exits with.
    pub(crate) const fn kind(self) -> ErrorKind {
        match self {
            Self::Compose => ErrorKind::NoInput,
            Self::Consistent => ErrorKind::DataFormat,
        }
    }

    /// Returns the one remediation template this check owns.
    pub(crate) const fn remediation(self) -> &'static str {
        match self {
            Self::Compose => {
                "{path}, named by profile {profile}, does not exist. Create it, correct \
                the name in the profile, or select a different profile."
            }
            Self::Consistent => {
                "The composed entry at {path} does not match the digest its inputs \
                recompute, so it was neither opened nor overwritten. Move it aside; the \
                next launch composes a fresh one. Report this — an entry is written once \
                and never rewritten."
            }
        }
    }

    /// Renders this check's failure as the four-part diagnostic.
    #[allow(clippy::literal_string_with_formatting_args)]
    pub(crate) fn diagnostic(self, path: &Path, profile: &str, why: &str) -> Diagnostic {
        let rendered = path.display().to_string();
        let hint = self
            .remediation()
            .replace("{path}", &rendered)
            .replace("{profile}", profile);
        Diagnostic::new(
            "the composed settings entry could not be used",
            rendered,
            format!("{}: {why}", self.id()),
            hint,
        )
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// Written out rather than derived. A table generated from the enum would
    /// agree with the enum by construction and prove nothing; this is the copy
    /// of the catalog the compiler can check.
    const STORAGE_CATALOG: &[(StorageCheck, &str)] = &[
        (StorageCheck::NoSymlinks, "storage-paths-no-symlinks"),
        (StorageCheck::Owned, "storage-paths-owned"),
        (StorageCheck::Typed, "storage-paths-typed"),
        (StorageCheck::DirectoryModes, "storage-directory-modes"),
        (StorageCheck::SecretModes, "storage-secret-modes"),
    ];

    #[test]
    fn every_check_id_maps_to_its_published_kind() {
        for (check, id) in STORAGE_CATALOG {
            assert_eq!(check.id(), *id);
            assert_eq!(check.kind(), ErrorKind::Permission);
            assert_eq!(check.kind().exit_code(), 77);
        }
    }

    /// A variant added without a row would otherwise be unchecked: the loop
    /// above only visits what the table already lists.
    #[test]
    fn the_catalog_covers_every_check() {
        let listed: std::collections::BTreeSet<&str> =
            STORAGE_CATALOG.iter().map(|(_, id)| *id).collect();
        assert_eq!(
            listed.len(),
            STORAGE_CATALOG.len(),
            "the catalog repeats a check"
        );
        assert_eq!(
            STORAGE_CATALOG.len(),
            5,
            "a storage check was added or removed without updating the catalog"
        );
    }

    #[test]
    fn complete_catalog_metadata_and_order_are_pinned() {
        let expected = [
            (
                "base-dirs-resolve",
                Scope::Host,
                Severity::Hard,
                ErrorKind::Unavailable,
            ),
            (
                "runtime-dir-present",
                Scope::Host,
                Severity::Soft,
                ErrorKind::Unavailable,
            ),
            (
                "wrapper-config-parses",
                Scope::Host,
                Severity::Hard,
                ErrorKind::Config,
            ),
            (
                "child-binary-resolves",
                Scope::Host,
                Severity::Hard,
                ErrorKind::ChildNotFound,
            ),
            (
                "child-is-executable",
                Scope::Host,
                Severity::Hard,
                ErrorKind::ChildNotExecutable,
            ),
            (
                "child-version-floor",
                Scope::Host,
                Severity::Soft,
                ErrorKind::Unavailable,
            ),
            (
                "storage-paths-no-symlinks",
                Scope::Session,
                Severity::Hard,
                ErrorKind::Permission,
            ),
            (
                "storage-paths-owned",
                Scope::Session,
                Severity::Hard,
                ErrorKind::Permission,
            ),
            (
                "storage-paths-typed",
                Scope::Session,
                Severity::Hard,
                ErrorKind::Permission,
            ),
            (
                "storage-directory-modes",
                Scope::Session,
                Severity::Hard,
                ErrorKind::Permission,
            ),
            (
                "storage-secret-modes",
                Scope::Session,
                Severity::Hard,
                ErrorKind::Permission,
            ),
            (
                "settings-compose",
                Scope::Session,
                Severity::Hard,
                ErrorKind::NoInput,
            ),
            (
                "settings-entry-consistent",
                Scope::Session,
                Severity::Hard,
                ErrorKind::DataFormat,
            ),
            (
                "account-registry-readable",
                Scope::Session,
                Severity::Soft,
                ErrorKind::Io,
            ),
            (
                "credentials-usable",
                Scope::Session,
                Severity::Soft,
                ErrorKind::Auth,
            ),
        ];
        assert_eq!(CATALOG.len(), 15);
        for (check, (id, scope, severity, kind)) in CATALOG.iter().zip(expected) {
            assert_eq!(
                (check.id(), check.scope(), check.severity(), check.kind()),
                (id, scope, severity, kind)
            );
        }
    }

    #[test]
    fn summary_preserves_the_first_hard_failure_in_catalog_order() {
        let warning = CheckResult::defect(Check::RuntimeDirPresent, "absent", String::new());
        let later = CheckResult::defect(Check::Entry(EntryCheck::Consistent), "bad", "fix".into());
        let earlier = CheckResult::defect(Check::WrapperConfigParses, "bad", "fix".into());
        let skipped = CheckResult::skipped(Check::Entry(EntryCheck::Compose), "inapplicable");
        assert_eq!(warning.kind(), Some(ErrorKind::Unavailable));
        let summary = Summary::fold(&[warning.clone(), earlier, later, skipped]);
        assert_eq!(summary.total, 4);
        assert_eq!(summary.warned, 1);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.hard_failures, 2);
        assert_eq!(summary.exit, 78);
        assert_eq!(summary.status(), CheckStatus::Fail);
        assert_eq!(Summary::fold(&[warning]).exit, 0);
    }

    #[test]
    fn a_failed_child_report_fails_the_verdict_without_strict() {
        let healthy = ChildReport {
            status: ChildStatus::Pass,
            output: Some(b"ok\n".to_vec()),
            exit: Some(0),
            reason: None,
        };
        let failed = ChildReport {
            status: ChildStatus::Fail,
            output: Some(b"broken\n".to_vec()),
            exit: Some(1),
            reason: None,
        };
        let clean = Summary::fold(&[CheckResult::pass(Check::BaseDirsResolve, "ok")]);
        assert_eq!(
            Verdict::fold(clean, &failed, false),
            Verdict {
                status: CheckStatus::Fail,
                exit: 69,
            }
        );
        assert_eq!(Verdict::fold(clean, &healthy, true).exit, 0);
        assert_eq!(
            Verdict::fold(clean, &ChildReport::skipped("not executable"), false).exit,
            0
        );
        // A wrapper hard failure keeps its own code, because it usually
        // explains the child's.
        let broken = Summary::fold(&[CheckResult::defect(
            Check::WrapperConfigParses,
            "bad",
            "fix".into(),
        )]);
        assert_eq!(Verdict::fold(broken, &failed, false).exit, 78);
        // Strict still promotes a wrapper warning, and nothing else.
        let warned = Summary::fold(&[CheckResult::defect(
            Check::RuntimeDirPresent,
            "absent",
            String::new(),
        )]);
        assert_eq!(Verdict::fold(warned, &healthy, false).exit, 0);
        assert_eq!(Verdict::fold(warned, &healthy, true).exit, 1);
        assert_eq!(
            Verdict::fold(warned, &healthy, true).status,
            CheckStatus::Warn
        );
    }

    #[test]
    fn a_remediation_substitutes_without_rewording() {
        let rendered = StorageCheck::Typed.diagnostic(
            Path::new("/s/claude-session/composed"),
            "directory",
            "regular file",
            None,
        );
        assert!(rendered.hint.contains("/s/claude-session/composed"));
        assert!(rendered.hint.contains("is a regular file"));
        assert!(rendered.hint.contains("must be a directory"));
        assert!(!rendered.hint.contains('{'), "{}", rendered.hint);
        assert!(rendered.why.contains("storage-paths-typed"));
    }

    /// The two entry checks exit differently, which is why they are two ids.
    #[test]
    fn every_entry_check_maps_to_its_published_kind() {
        assert_eq!(EntryCheck::Compose.id(), "settings-compose");
        assert_eq!(EntryCheck::Compose.kind(), ErrorKind::NoInput);
        assert_eq!(EntryCheck::Consistent.id(), "settings-entry-consistent");
        assert_eq!(EntryCheck::Consistent.kind(), ErrorKind::DataFormat);
    }

    #[test]
    fn an_entry_remediation_substitutes_the_path_and_profile() {
        let rendered = EntryCheck::Compose.diagnostic(
            Path::new("/c/claude-session/settings/base.json"),
            "work",
            "the piece does not exist",
        );
        assert!(rendered.hint.contains("settings/base.json"));
        assert!(rendered.hint.contains("profile work"));
        assert!(!rendered.hint.contains('{'), "{}", rendered.hint);
        assert!(rendered.why.starts_with("settings-compose: "));
    }

    #[test]
    fn a_mode_remediation_renders_the_octal_mode() {
        let rendered = StorageCheck::DirectoryModes.diagnostic(
            Path::new("/s/claude-session/accounts"),
            "directory",
            "directory",
            Some(0o700),
        );
        assert!(rendered.hint.contains("0700"), "{}", rendered.hint);
        assert!(!rendered.hint.contains('{'), "{}", rendered.hint);
    }
}
