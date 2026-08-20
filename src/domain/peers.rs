//! Naming the peer-registry scope one run's sessions share.
//!
//! This module is for turning what the operating system says about a run's
//! kernel boot and mount namespace into the two path components under
//! `peers/`. It performs no read — that is `adapters::host` — and decides no
//! path, which is `domain::paths`.
//!
//! The scope answers one question, whether a listed peer is reachable, and
//! each component answers half of it: distinct kernels share their initial
//! mount-namespace inode but never a boot identifier, and containers of one
//! kernel share its boot but not their mount namespaces ([ADR-0108]). The
//! namespace component fingerprints the bare link, without [ADR-0109]'s
//! per-kernel discriminator: the boot component already separates kernels,
//! and folding the machine identifier in could only split peers that share a
//! boot and a mount namespace — reachable peers — over a `chroot`'s different
//! view of it.
//!
//! [ADR-0108]: ../../docs/decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md
//! [ADR-0109]: ../../docs/decisions/ADR-0109-discriminate-namespaces-across-kernels.md

use std::{ffi::OsStr, os::unix::ffi::OsStrExt as _, str::FromStr as _};

use crate::domain::{
    identifier::Identifier,
    namespace::{Kind, fingerprint},
};

/// The hex digits of the boot identifier a scope keeps.
///
/// Forty-eight random bits: enough that two kernels on one machine colliding
/// is not something to plan around, short enough that the component joins the
/// identifier grammar with room to spare.
const BOOT_DIGITS: usize = 12;

/// One run's peer scope: the boot component and the namespace under it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Scope {
    boot: Identifier,
    namespace: Identifier,
}

impl Scope {
    /// Derives a scope from the kernel's boot identifier and mount-namespace
    /// link.
    ///
    /// The boot identifier is a UUID, so its hyphens are dropped and the first
    /// [`BOOT_DIGITS`] hex digits name the boot. A value that is not hex all
    /// the way through, or too short once cleaned, names nothing — the caller
    /// launches unshared rather than sharing under a name another boot could
    /// also produce. The namespace component fingerprints the bare link, and
    /// an empty link likewise names nothing.
    pub(crate) fn derive(boot_id: &str, link: &OsStr) -> Option<Self> {
        let cleaned: String = boot_id
            .trim()
            .chars()
            .filter(|character| *character != '-')
            .map(|character| character.to_ascii_lowercase())
            .collect();
        if cleaned.len() < BOOT_DIGITS || !cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        if link.as_bytes().is_empty() {
            return None;
        }
        let boot = Identifier::from_str(&format!("boot-{}", &cleaned[..BOOT_DIGITS])).ok()?;
        let namespace = fingerprint(Kind::Mount.procfs_name(), link.as_bytes())?;
        Some(Self { boot, namespace })
    }

    /// Borrows the boot path component.
    pub(crate) const fn boot(&self) -> &Identifier {
        &self.boot
    }

    /// Borrows the namespace path component.
    pub(crate) const fn namespace(&self) -> &Identifier {
        &self.namespace
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn link(value: &str) -> OsString {
        OsString::from(value)
    }

    #[test]
    fn a_boot_identifier_becomes_one_component() {
        let scope = Scope::derive(
            "3f2a9c1d-58aa-4b0e-9c1d-0a1b2c3d4e5f",
            &link("mnt:[4026531840]"),
        )
        .expect("a UUID names a boot");
        assert_eq!(scope.boot().as_str(), "boot-3f2a9c1d58aa");
        assert!(scope.namespace().as_str().starts_with("mnt-"));
    }

    #[test]
    fn a_different_boot_or_namespace_scopes_apart() {
        let one = Scope::derive(
            "3f2a9c1d-58aa-4b0e-9c1d-0a1b2c3d4e5f",
            &link("mnt:[4026531840]"),
        )
        .expect("scope");
        let other_boot = Scope::derive(
            "77febc02-1111-4b0e-9c1d-0a1b2c3d4e5f",
            &link("mnt:[4026531840]"),
        )
        .expect("scope");
        let other_namespace = Scope::derive(
            "3f2a9c1d-58aa-4b0e-9c1d-0a1b2c3d4e5f",
            &link("mnt:[4026533427]"),
        )
        .expect("scope");
        assert_ne!(one.boot(), other_boot.boot());
        assert_eq!(one.namespace(), other_boot.namespace());
        assert_eq!(one.boot(), other_namespace.boot());
        assert_ne!(one.namespace(), other_namespace.namespace());
    }

    /// The scope is a pure function of boot and link: the machine identifier
    /// that discriminates session directories does not enter it, so two views
    /// of one boot and mount namespace always share one registry ([ADR-0108]).
    ///
    /// [ADR-0108]: ../../docs/decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md
    #[test]
    fn one_boot_and_namespace_always_name_one_scope() {
        let one = Scope::derive(
            "3f2a9c1d-58aa-4b0e-9c1d-0a1b2c3d4e5f",
            &link("mnt:[4026531840]"),
        )
        .expect("scope");
        let same = Scope::derive(
            "3f2a9c1d-58aa-4b0e-9c1d-0a1b2c3d4e5f",
            &link("mnt:[4026531840]"),
        )
        .expect("scope");
        assert_eq!(one, same);
    }

    #[test]
    fn an_underivable_boot_scopes_nothing() {
        for value in ["", "not-hex-at-all", "abc123", "   "] {
            assert!(
                Scope::derive(value, &link("mnt:[4026531840]")).is_none(),
                "{value:?} should name nothing"
            );
        }
        assert!(
            Scope::derive("3f2a9c1d-58aa-4b0e-9c1d-0a1b2c3d4e5f", &link("")).is_none(),
            "an empty link should name nothing"
        );
    }

    #[test]
    fn surrounding_whitespace_and_case_do_not_change_the_name() {
        let bare =
            Scope::derive("3F2A9C1D-58AA-4B0E-9C1D-0A1B2C3D4E5F", &link("mnt:[1]")).expect("scope");
        assert_eq!(bare.boot().as_str(), "boot-3f2a9c1d58aa");
    }
}
