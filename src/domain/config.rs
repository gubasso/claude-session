//! Immutable resolved configuration and provenance.

use std::path::PathBuf;

use super::identifier::Identifier;

/// The layer which supplied a resolved configuration value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Source {
    /// No layer supplied a value.
    Default,
    /// The user configuration file supplied the value.
    User,
    /// The selected account's own binding supplied the value.
    ///
    /// Above the user file and below the project file: the binding is the
    /// account's durable intent, and a project file is a deliberate per-tree
    /// override of it
    /// ([ADR-0096](../../docs/decisions/ADR-0096-bind-a-profile-to-an-account.md)).
    Account,
    /// The project configuration file supplied the value.
    Project,
    /// The process environment supplied the value.
    Environment,
    /// The command line supplied the value.
    Cli,
}

impl Source {
    /// Returns the spelling both report forms print.
    ///
    /// Part of a verb's output schema rather than a display detail, so it has
    /// one owner instead of one per renderer.
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::User => "user-config",
            Self::Account => "account",
            Self::Project => "project-config",
            Self::Environment => "environment",
            Self::Cli => "cli",
        }
    }
}

/// A value paired with its winning configuration source.
#[derive(Clone, Debug)]
pub(crate) struct Sourced<T> {
    value: Option<T>,
    source: Source,
}

impl<T> Sourced<T> {
    /// Constructs an unset default value.
    pub(crate) const fn unset() -> Self {
        Self {
            value: None,
            source: Source::Default,
        }
    }

    /// Replaces the value and records its source.
    pub(crate) fn set(&mut self, value: T, source: Source) {
        self.value = Some(value);
        self.source = source;
    }

    /// Borrows the resolved value.
    pub(crate) const fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Returns the winning source.
    pub(crate) const fn source(&self) -> Source {
        self.source
    }
}

/// All wrapper configuration after precedence has been applied.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedConfig {
    child_bin: Sourced<PathBuf>,
    default_account: Sourced<Identifier>,
    default_profile: Sourced<Identifier>,
}

impl ResolvedConfig {
    /// Constructs the all-unset built-in defaults.
    pub(crate) const fn defaults() -> Self {
        Self {
            child_bin: Sourced::unset(),
            default_account: Sourced::unset(),
            default_profile: Sourced::unset(),
        }
    }

    /// Returns the configured child override.
    pub(crate) const fn child_bin(&self) -> Option<&PathBuf> {
        self.child_bin.value()
    }
    /// Returns the child override provenance.
    pub(crate) const fn child_bin_source(&self) -> Source {
        self.child_bin.source()
    }
    /// Mutably accesses the child value during resolution only.
    pub(crate) const fn child_bin_mut(&mut self) -> &mut Sourced<PathBuf> {
        &mut self.child_bin
    }
    /// Returns the selected account, if any layer supplied one.
    pub(crate) const fn account(&self) -> Option<&Identifier> {
        self.default_account.value()
    }
    /// Returns the account selection provenance.
    pub(crate) const fn account_source(&self) -> Source {
        self.default_account.source()
    }
    /// Returns the profile selection provenance.
    pub(crate) const fn profile_source(&self) -> Source {
        self.default_profile.source()
    }
    /// Returns the selected profile, if any layer supplied one.
    pub(crate) const fn profile(&self) -> Option<&Identifier> {
        self.default_profile.value()
    }
    /// Mutably accesses the account value during resolution only.
    pub(crate) const fn account_mut(&mut self) -> &mut Sourced<Identifier> {
        &mut self.default_account
    }
    /// Mutably accesses the profile value during resolution only.
    pub(crate) const fn profile_mut(&mut self) -> &mut Sourced<Identifier> {
        &mut self.default_profile
    }
}
