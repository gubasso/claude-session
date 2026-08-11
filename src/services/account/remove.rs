//! Ordered removal of one account's local state.
//!
//! The order inverts the rotation order, and that inversion is the design. A
//! rotation commits when `auth-mode.json` lands; a removal commits when it
//! goes, because an account without its mode metadata is not an account. So
//! whatever interrupts a removal after the first unlink leaves a directory that
//! `account list` reports as `invalid` and a launch refuses — legible residue
//! rather than a usable account with half its state gone.
//!
//! The lock is taken before anything is deleted, so a removal cannot interleave
//! with a concurrent `account login`, and it is released only after the tree is
//! gone. The sentinel lives beside the account and is not touched here: an
//! earlier design deleted it with the tree, which left a window between that
//! unlink and the `rmdir` in which acquisition would create a second inode and
//! exclude nobody ([ADR-0087]).
//!
//! [ADR-0087]: ../../../docs/decisions/ADR-0087-keep-the-credential-lock-beside-the-account.md

use std::{fs, path::Path};

use crate::{
    context::AppContext,
    domain::{
        account::{AuthMode, Removal, ReportMode},
        identifier::Identifier,
    },
    error::{AppError, Diagnostic, ErrorKind},
    services::{account::lock::CredentialLock, storage::guard},
};

/// Removes one account tree in the order the commit depends on.
///
/// The lock arrives by value because holding it is a precondition worth stating
/// in the type rather than in a comment, and because dropping it here — after
/// the directory is gone — is what closes the descriptor last.
pub(crate) fn perform(
    context: &AppContext,
    account: &Identifier,
    lock: CredentialLock,
) -> Result<Removal, AppError> {
    let paths = context.paths();
    let directory = paths.account(account);
    // Refuse a link or a foreign owner before deleting anything: a removal that
    // followed a symbolic link would delete something the user never named.
    guard::validate(paths.state(), &directory, guard::Expected::Directory)?;
    let mode = super::read_metadata(paths.state(), &paths.account_auth_mode(account))
        .ok()
        .map_or(ReportMode::Invalid, |metadata| match metadata.mode {
            AuthMode::Login => ReportMode::Login,
            AuthMode::Token => ReportMode::Token,
        });

    // The commit.
    unlink(&paths.account_auth_mode(account))?;

    // Immediately after the commit, because from this instant the marker names
    // something that is no longer an account. A failure here is warned and
    // carried in the report: a courtesy write must not fail a verb that has
    // already committed.
    let marker_cleared = clear_marker(context, account);

    unlink(&paths.account_oauth_token(account))?;
    remove_contents(&directory)?;
    fs::remove_dir(&directory).map_err(|error| {
        residue(
            "the account directory could not be removed",
            &directory,
            &error,
        )
    })?;
    // Explicit, and last: the descriptor closes only after the tree it guarded
    // is gone, so nothing acquires and starts writing into a half-removed
    // directory.
    drop(lock);
    Ok(Removal {
        account: account.clone(),
        path: directory,
        removed: true,
        mode: Some(mode),
        marker_cleared: Some(marker_cleared),
    })
}

/// Removes everything beneath the account directory.
///
/// The order within this step is free — nothing below carries a commit — so the
/// unordered recursive remover is reused rather than reimplemented.
fn remove_contents(directory: &Path) -> Result<(), AppError> {
    let entries =
        crate::adapters::filesystem::SystemFileSystem::dir_entries(directory).map_err(|error| {
            residue(
                "the account contents could not be listed",
                directory,
                &error,
            )
        })?;
    for name in entries {
        let child = directory.join(&name);
        let facts = crate::adapters::filesystem::SystemFileSystem::look(&child)
            .map_err(|error| residue("an account entry could not be inspected", &child, &error))?;
        let result = match facts {
            None => Ok(()),
            Some(facts) if facts.directory && !facts.symlink => super::remove_tree(&child),
            Some(_) => fs::remove_file(&child),
        };
        result.map_err(|error| residue("an account entry could not be removed", &child, &error))?;
    }
    Ok(())
}

/// Unlinks one path, treating an absent one as already done.
fn unlink(path: &Path) -> Result<(), AppError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(residue(
            "an account file could not be removed",
            path,
            &error,
        )),
    }
}

/// Clears the last-used marker when it names the removed account.
///
/// Returns whether it was cleared. A read failure is treated as "the marker
/// does not name this account", because the alternative — deleting a marker
/// this run could not read — would clear a selection the user still wants.
fn clear_marker(context: &AppContext, account: &Identifier) -> bool {
    let marker = context.paths().last_account();
    let names_this_account = super::marker_names(context, account);
    if !names_this_account {
        return false;
    }
    fs::remove_file(&marker).is_ok()
}

/// Reports a failure that left the account partly removed.
///
/// The path is named exactly, because after the commit the user is the only one
/// who can finish the job and a directory name is what they need to do it.
fn residue(what: &str, path: &Path, error: &std::io::Error) -> AppError {
    AppError::new(
        ErrorKind::Io,
        Diagnostic::new(
            what,
            path.display().to_string(),
            error.to_string(),
            "the account is already unusable; remove the remaining directory by hand",
        ),
    )
}
