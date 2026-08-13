//! One account's profile binding: reading it, writing it, and its ladder rung.
//!
//! The binding is the account's own answer to "which settings do I run under"
//! ([ADR-0096](../../../docs/decisions/ADR-0096-bind-a-profile-to-an-account.md)).
//! It is the wrapper's file and the wrapper's shape, so it is written by one
//! verb that validates rather than by hand.

use std::{io::Read as _, path::Path};

use crate::{
    adapters::{
        clock::{Clock as _, rfc3339_utc},
        filesystem::SystemFileSystem,
    },
    context::AppContext,
    domain::{
        account::{ProfileBinding, RecordedAt},
        config::{ResolvedConfig, Source},
        identifier::Identifier,
        paths::XdgPaths,
    },
    error::{AppError, Diagnostic, ErrorKind},
    services::storage::{atomic, guard},
};

/// Reads one account's binding, separating a safety refusal from an absent one.
///
/// A content defect is an absent binding: returning an error for a document the
/// wrapper can rewrite would take a bootstrap that only wanted to resolve a
/// profile and turn it into a failure of every verb, and `account status` is
/// the verb asked about the account's health.
///
/// A storage-safety refusal is not a content defect. It is the case
/// [XDG storage](../../../docs/reference/xdg-storage.md) requires be refused
/// rather than absorbed, and absorbing it here would let a symlinked or
/// foreign-owned binding read as merely unbound and fall through to the user
/// file, which is the silent resolution the guard exists to prevent.
pub(crate) fn read(
    paths: &XdgPaths,
    account: &Identifier,
) -> Result<Option<ProfileBinding>, AppError> {
    let path = paths.account_profile(account);
    guard::validate(paths.state(), &path, guard::Expected::PrivateFile)?;
    let Ok(mut handle) = SystemFileSystem::open_private_file(&path) else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    if handle.read_to_end(&mut bytes).is_err() {
        return Ok(None);
    }
    Ok(serde_json::from_slice(&bytes).ok())
}

/// Applies the binding rung to a resolved configuration.
///
/// Only where the profile is unset or came from the user file: the flag, the
/// environment, and the project file each outrank an account's durable intent,
/// and each is a choice somebody made for this invocation or this tree.
/// A binding that cannot be read safely applies no rung, and is reported rather
/// than raised.
///
/// This runs during the bootstrap of every invocation, before the verb is
/// known, so a refusal here would take a storage defect and turn it into a
/// failure of `doctor` — the verb whose whole job is to find that defect and
/// say what to do about it — along with every other verb. Nothing unsafe is
/// read: the guard refuses, the rung is skipped, and the profile keeps whatever
/// the user file or the default gave it. `account status` and `doctor` are the
/// verbs that name the condition, and they do.
pub(crate) fn apply(config: &mut ResolvedConfig, account: Option<&Identifier>, paths: &XdgPaths) {
    if !matches!(config.profile_source(), Source::Default | Source::User) {
        return;
    }
    let Some(account) = account else { return };
    let Some(binding) = read(paths, account).ok().flatten() else {
        return;
    };
    config.profile_mut().set(binding.profile, Source::Account);
}

/// Binds an existing account to an existing profile.
///
/// Both halves are checked before anything is written, so a typo leaves the
/// previous binding in place rather than replacing a working one with a name
/// that resolves to nothing
/// ([ADR-0097](../../../docs/decisions/ADR-0097-rebind-a-profile-without-re-authenticating.md)).
pub(crate) fn perform(
    context: &AppContext,
    account: &Identifier,
    profile: &Identifier,
) -> Result<ProfileBinding, AppError> {
    super::status::require_existing(context, account)?;
    require_profile(context, profile)?;
    let binding = ProfileBinding {
        profile: profile.clone(),
        recorded_at: RecordedAt::new_unchecked(rfc3339_utc(context.adapters().clock().now())),
    };
    let _lock = super::hold(context, account)?;
    write(context, account, &binding)?;
    Ok(binding)
}

/// Encodes and atomically writes one account's binding.
pub(crate) fn write(
    context: &AppContext,
    account: &Identifier,
    binding: &ProfileBinding,
) -> Result<(), AppError> {
    let mut bytes = serde_json::to_vec(binding).map_err(|error| {
        AppError::new(
            ErrorKind::Internal,
            Diagnostic::new(
                "profile binding could not be encoded",
                account.as_str(),
                error.to_string(),
                "report this invariant",
            ),
        )
    })?;
    bytes.push(b'\n');
    let path = context.paths().account_profile(account);
    guard::validate(context.paths().state(), &path, guard::Expected::PrivateFile)?;
    atomic::write(&path, &bytes, 0o600)
}

/// Reports whether a profile has a document to compose from.
pub(crate) fn profile_present(context: &AppContext, profile: &Identifier) -> bool {
    exists(&context.paths().profile_file(profile))
}

/// Refuses a profile that has no document.
///
/// `NoInput` rather than `Usage`: the name is well formed and simply names
/// nothing, which is the same answer a launch gives for an unresolvable profile.
fn require_profile(context: &AppContext, profile: &Identifier) -> Result<(), AppError> {
    if profile_present(context, profile) {
        return Ok(());
    }
    Err(AppError::new(
        ErrorKind::NoInput,
        Diagnostic::new(
            "no such profile",
            context.paths().profile_file(profile).display().to_string(),
            format!("no profile named {} exists locally", profile.as_str()),
            "run claude-session-rs profile to see the profiles that do",
        ),
    ))
}

fn exists(path: &Path) -> bool {
    SystemFileSystem::look(path)
        .is_ok_and(|facts| facts.is_some_and(|facts| !facts.symlink && !facts.directory))
}
