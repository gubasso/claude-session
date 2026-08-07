//! The session half of the probe catalog: stable ids and their remediations.
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
    const CATALOG: &[(StorageCheck, &str)] = &[
        (StorageCheck::NoSymlinks, "storage-paths-no-symlinks"),
        (StorageCheck::Owned, "storage-paths-owned"),
        (StorageCheck::Typed, "storage-paths-typed"),
        (StorageCheck::DirectoryModes, "storage-directory-modes"),
        (StorageCheck::SecretModes, "storage-secret-modes"),
    ];

    #[test]
    fn every_check_id_maps_to_its_published_kind() {
        for (check, id) in CATALOG {
            assert_eq!(check.id(), *id);
            assert_eq!(check.kind(), ErrorKind::Permission);
            assert_eq!(check.kind().exit_code(), 77);
        }
    }

    /// A variant added without a row would otherwise be unchecked: the loop
    /// above only visits what the table already lists.
    #[test]
    fn the_catalog_covers_every_check() {
        let listed: std::collections::BTreeSet<&str> = CATALOG.iter().map(|(_, id)| *id).collect();
        assert_eq!(listed.len(), CATALOG.len(), "the catalog repeats a check");
        assert_eq!(
            CATALOG.len(),
            5,
            "a storage check was added or removed without updating the catalog"
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
