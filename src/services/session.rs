//! Resolving this run's account and profile selection into paths.
//!
//! This module is for turning a selection into paths and preparing them on
//! first use. It is not for authentication, and it puts nothing on the child's
//! argument vector.

use std::path::PathBuf;

use crate::{context::AppContext, domain::identifier::Identifier};

/// The paths one run's selection resolves to.
///
/// Pure: building this reads configuration and computes paths, and touches no
/// filesystem. That is what makes it safe to hold on the immutable context,
/// where a validation result would not be.
#[derive(Clone, Debug)]
pub(crate) struct SessionPaths {
    account: Option<Account>,
    profile: Option<Identifier>,
}

/// One account's two directories.
#[derive(Clone, Debug)]
pub(crate) struct Account {
    /// The account identifier.
    pub(crate) id: Identifier,
    /// The account directory.
    pub(crate) directory: PathBuf,
    /// The child-owned native configuration directory.
    pub(crate) config: PathBuf,
}

impl SessionPaths {
    /// Resolves the selection without touching the filesystem.
    pub(crate) fn resolve(context: &AppContext) -> Self {
        let paths = context.paths();
        Self {
            account: context.account_selection().account().map(|id| Account {
                id: id.clone(),
                directory: paths.account(id),
                config: paths.account_config(id),
            }),
            profile: context.config().profile().cloned(),
        }
    }

    /// Returns the selected account, if any.
    pub(crate) const fn account(&self) -> Option<&Account> {
        self.account.as_ref()
    }
    /// Returns the selected profile, if any.
    pub(crate) const fn profile(&self) -> Option<&Identifier> {
        self.profile.as_ref()
    }
}
