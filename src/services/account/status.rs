//! One account's status projection.
//!
//! This is the verb that was asked about a credential, so it is the only one
//! that opens the wrapper-owned token: `account list` answers from local state
//! alone, because one probe per account would be one child per account, and a
//! launch reads the token without judging it. Consistency is a status fact.

use crate::{
    context::AppContext,
    domain::{
        account::{AccountStatus, AuthMode, AuthModeMetadata, Probe, ReportMode, Warning},
        identifier::Identifier,
        secret::Fingerprint,
    },
    error::{AppError, ErrorKind},
    services::{account::token, storage::guard},
};

/// Builds the status of one named account.
///
/// Every finding is data. The only failure this can return is one that says
/// nothing about the account's health — a storage-safety refusal or an
/// unreadable tree — because the exit of an inspection verb answers "did the
/// verb run", not "is the credential good".
pub(crate) fn project(
    context: &AppContext,
    account: &Identifier,
    selected: bool,
) -> Result<AccountStatus, AppError> {
    let paths = context.paths();
    guard::validate(
        paths.state(),
        &paths.account(account),
        guard::Expected::Directory,
    )?;
    let metadata = read(context, account);
    let child_login_present = super::owned_regular(&paths.account_credentials(account));
    let mut status = AccountStatus {
        account: account.clone(),
        selected,
        mode: ReportMode::Invalid,
        usable: false,
        warnings: Vec::new(),
        selection_source: selected.then(|| context.account_selection().source()),
        recorded_at: None,
        age_seconds: None,
        estimated_expiry: None,
        fingerprint: None,
        metadata_consistent: None,
        child_login_present: Some(child_login_present),
        child_probe: None,
        profile: None,
        profile_present: None,
        // The layer that supplied this run's profile, which is a fact about
        // the run rather than about the account, so only the selected account
        // is the subject of it.
        profile_source: selected.then(|| context.config().profile_source()),
        effective_profile: selected
            .then(|| context.config().profile().cloned())
            .flatten(),
    };
    // A safety refusal on the binding leaves by the same door the directory
    // walk above uses: it says nothing about the credential's health, which is
    // the only kind of failure this projection is allowed to return.
    if let Some(binding) = super::bind::read(paths, account)? {
        let present = super::bind::profile_present(context, &binding.profile);
        status.profile = Some(binding.profile);
        status.profile_present = Some(present);
    }
    let Some(metadata) = metadata else {
        // `invalid` is a report value and never durable metadata: the directory
        // exists but nothing says what it is, which is exactly what the mode
        // this reports means.
        status.warnings = super::launch_warnings(context, account, ReportMode::Invalid);
        note_missing_profile(&mut status);
        note_first_run(context, account, &mut status);
        return Ok(status);
    };
    status.recorded_at = Some(metadata.recorded_at.clone());
    match metadata.mode {
        AuthMode::Login => {
            status.mode = ReportMode::Login;
            status.usable = child_login_present;
            project_age(context, &metadata, &mut status);
        }
        AuthMode::Token => project_token(context, account, &metadata, &mut status),
    }
    // The same detection the pre-launch line and `doctor` use, so a condition
    // is worded once and a machine consumer of `--json` sees exactly what a
    // human running the launch would have read on standard error.
    status.warnings = super::launch_warnings(context, account, status.mode);
    note_missing_profile(&mut status);
    note_first_run(context, account, &mut status);
    Ok(status)
}

/// Adds the bound-but-absent profile to the warnings the launch would raise.
///
/// Appended after the credential warnings rather than mixed into them, because
/// `launch_warnings` answers one question — which credential wins — and this
/// answers another.
fn note_missing_profile(status: &mut AccountStatus) {
    if status.profile_present == Some(false) {
        status.warnings.push(Warning::BoundProfileMissing);
    }
}

/// Adds the first-run condition to the warnings the launch would raise.
///
/// Its own note for the reason the one above is: this answers neither which
/// credential wins nor which profile applies, but whether the child would show
/// its prompt at all. An unreadable file warns too — the reader is going to meet
/// the same wizard either way, and `doctor` is where the distinction earns its
/// own words.
fn note_first_run(context: &AppContext, account: &Identifier, status: &mut AccountStatus) {
    if !super::onboarding::readiness(context, account).ready() {
        status.warnings.push(Warning::FirstRunOnboarding);
    }
}

/// Fills in the token-only fields, including the torn-pair case.
///
/// The reported fingerprint is always the recomputed one, because the recorded
/// one describes a token that is no longer on disk. `metadata_consistent` is
/// what says the two disagree; the fingerprint says which credential is in
/// force.
fn project_token(
    context: &AppContext,
    account: &Identifier,
    metadata: &AuthModeMetadata,
    status: &mut AccountStatus,
) {
    status.mode = ReportMode::Token;
    let Some(recorded) = metadata.token_fingerprint() else {
        // Token metadata without a fingerprint is malformed rather than
        // fingerprint-free, so it reports as unusable instead of getting a
        // default that would claim a consistency nobody checked.
        status.mode = ReportMode::Invalid;
        return;
    };
    let Ok(stored) = token::read_stored(context, account) else {
        return;
    };
    let actual = Fingerprint::of(&stored);
    let consistent = actual == *recorded;
    status.usable = true;
    status.metadata_consistent = Some(consistent);
    status.fingerprint = Some(actual);
    if consistent {
        project_age(context, metadata, status);
    }
    // A launch would still work here, so the probe is asked either way: the
    // token was proven once and the metadata is what drifted.
    status.child_probe = probe(context, account, &stored);
}

fn project_age(context: &AppContext, metadata: &AuthModeMetadata, status: &mut AccountStatus) {
    if metadata.mode == AuthMode::Token {
        status.age_seconds = token::age_seconds(context, &metadata.recorded_at);
        status.estimated_expiry = token::estimated_expiry(&metadata.recorded_at);
    }
}

/// Asks the child about the stored token, if a child can be resolved.
///
/// An unresolvable child is `unavailable` rather than an error, because a
/// status report that refused to render over a missing `claude` would withhold
/// every local fact the user came for.
fn probe(
    context: &AppContext,
    account: &Identifier,
    stored: &crate::domain::secret::Secret,
) -> Option<Probe> {
    let program = crate::services::child::program(context).ok()?;
    Some(token::probe(context, account, &program, stored))
}

/// Reads the mode metadata, treating any defect as an absent mode.
///
/// A status report describes what it found rather than refusing over it, so a
/// malformed document becomes `mode: invalid` here instead of an error. The
/// storage-safety walk above already ran, so a defect this swallows is a
/// content problem and not a security one.
fn read(context: &AppContext, account: &Identifier) -> Option<AuthModeMetadata> {
    let path = context.paths().account_auth_mode(account);
    super::read_metadata(context.paths().state(), &path).ok()
}

/// Locates a named account, or reports that it does not exist.
pub(crate) fn require_existing(context: &AppContext, account: &Identifier) -> Result<(), AppError> {
    let directory = context.paths().account(account);
    let present = crate::adapters::filesystem::SystemFileSystem::look(&directory)
        .map_err(|error| {
            AppError::new(
                ErrorKind::Io,
                crate::error::Diagnostic::new(
                    "account path could not be inspected",
                    directory.display().to_string(),
                    error.to_string(),
                    "check the account path",
                ),
            )
        })?
        .is_some();
    if present {
        Ok(())
    } else {
        Err(AppError::new(
            ErrorKind::NoInput,
            crate::error::Diagnostic::new(
                "no such account",
                directory.display().to_string(),
                format!("no account named {} exists locally", account.as_str()),
                "run claude-session-rs account list to see the accounts that do",
            ),
        ))
    }
}

/// Resolves which account a status request is about.
pub(crate) fn subject(
    context: &AppContext,
    name: Option<Identifier>,
) -> Result<(Identifier, bool), AppError> {
    let selected = context.account_selection().account().cloned();
    match name {
        Some(named) => {
            let is_selected = selected.as_ref() == Some(&named);
            Ok((named, is_selected))
        }
        None => selected.map(|account| (account, true)).ok_or_else(|| {
            AppError::new(
                ErrorKind::Usage,
                crate::error::Diagnostic::new(
                    "account status needs an account",
                    "account status",
                    "no name or selected account was available",
                    "pass an account name or select one with --account",
                ),
            )
        }),
    }
}
