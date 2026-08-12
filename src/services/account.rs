//! Account selection, discovery, local usability, and launch guards.

pub(crate) mod lock;
pub(crate) mod remove;
pub(crate) mod status;
pub(crate) mod token;

use std::{fs, io::Read as _, path::Path, str::FromStr};

use crate::{
    adapters::{
        clock::{Clock, rfc3339_utc},
        environment::Environment as _,
        filesystem::SystemFileSystem,
        process::ProcessRunner,
    },
    context::AppContext,
    domain::{
        account::{
            AccountFinding, AccountSelection, AmbientCredential, AuthMode, AuthModeMetadata,
            RecordedAt, ReportMode, SelectionSource, Warning,
        },
        checks::{AccountCheck, Check, CheckResult},
        child::{ChildInvocation, ChildOutcome, ChildVersion, MINIMUM_CHILD_VERSION},
        config::ResolvedConfig,
        identifier::Identifier,
        paths::XdgPaths,
    },
    error::{AppError, Diagnostic, ErrorKind},
};

use super::storage::{atomic, guard};

pub(crate) fn resolve_selection(
    config: &ResolvedConfig,
    paths: &XdgPaths,
) -> Result<AccountSelection, AppError> {
    if let Some(account) = config.account() {
        return Ok(AccountSelection::new(
            account.clone(),
            config.account_source().into(),
        ));
    }
    let marker = paths.last_account();
    guard::validate(paths.state(), &marker, guard::Expected::PrivateFile)?;
    let mut handle = match SystemFileSystem::open_private_file(&marker) {
        Ok(handle) => handle,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AccountSelection::none());
        }
        Err(error) => {
            return Err(path_error(
                "last-used account marker is unavailable",
                &marker,
                &error,
            ));
        }
    };
    let mut bytes = Vec::new();
    handle.read_to_end(&mut bytes).map_err(|error| {
        path_error(
            "last-used account marker could not be read",
            &marker,
            &error,
        )
    })?;
    let text = std::str::from_utf8(&bytes).map_err(|error| {
        AppError::new(
            ErrorKind::DataFormat,
            Diagnostic::new(
                "last-used account marker is malformed",
                marker.display().to_string(),
                error.to_string(),
                "remove the marker or select an account explicitly",
            ),
        )
    })?;
    let account = Identifier::from_str(text).map_err(|error| {
        AppError::new(
            ErrorKind::DataFormat,
            Diagnostic::new(
                "last-used account marker is malformed",
                marker.display().to_string(),
                error.to_string(),
                "remove the marker or select an account explicitly",
            ),
        )
    })?;
    Ok(AccountSelection::new(account, SelectionSource::Marker))
}

pub(crate) fn discover(context: &AppContext) -> Result<Vec<AccountFinding>, AppError> {
    let root = context.paths().accounts();
    match SystemFileSystem::look(&root) {
        Ok(None) => return Ok(Vec::new()),
        Ok(Some(facts))
            if !facts.symlink
                && facts.directory
                && facts.uid == rustix::process::getuid().as_raw() => {}
        Ok(Some(_)) => {
            return Err(AppError::new(
                ErrorKind::Permission,
                Diagnostic::new(
                    "account collection is unsafe",
                    root.display().to_string(),
                    "the path is a link, another type, or owned by another user",
                    "move the path aside and let account login recreate it",
                ),
            ));
        }
        Err(error) => {
            return Err(path_error(
                "account collection could not be inspected",
                &root,
                &error,
            ));
        }
    }
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(path_error(
                "account collection could not be read",
                &root,
                &error,
            ));
        }
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry
            .map_err(|error| path_error("account collection could not be read", &root, &error))?;
        let name = entry.file_name();
        let Some(text) = name.to_str() else { continue };
        let Ok(id) = text.parse::<Identifier>() else {
            continue;
        };
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() && !kind.is_symlink() {
            names.push(id);
        }
    }
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    Ok(names
        .into_iter()
        .map(|name| inspect(context, name))
        .collect())
}

#[allow(
    clippy::option_if_let_else,
    reason = "the two doctor skip and evaluation branches mirror catalog semantics"
)]
pub(crate) fn doctor_results(context: &AppContext) -> [CheckResult; 2] {
    let registry = Check::Account(AccountCheck::RegistryReadable);
    let credentials = Check::Account(AccountCheck::CredentialsUsable);
    match discover(context) {
        Err(error) => [
            CheckResult::defect(
                registry,
                error.diagnostic().why.clone(),
                registry
                    .hint(&[("path", &context.paths().accounts().display().to_string())])
                    .unwrap_or_default(),
            ),
            CheckResult::skipped(credentials, "the account collection is unreadable"),
        ],
        Ok(accounts) if accounts.is_empty() => [
            CheckResult::skipped(
                registry,
                "no account collection or account directories exist",
            ),
            CheckResult::skipped(credentials, "no account exists"),
        ],
        Ok(accounts) => {
            let registry_result = CheckResult::pass(
                registry,
                format!("discovered {} account directories", accounts.len()),
            );
            let credentials_result = match context.account_selection().account() {
                None => CheckResult::skipped(credentials, "no account is selected"),
                Some(selected) => match accounts.iter().find(|account| &account.name == selected) {
                    // Mode-aware: a token account's usable artifact is the
                    // wrapper-owned token, and `usable` already knows which
                    // artifact each mode requires. Requiring login mode here
                    // would report every healthy token account as broken.
                    Some(account) if account.usable => {
                        let warnings = launch_warnings(context, selected, account.mode);
                        if warnings.is_empty() {
                            CheckResult::pass(
                                credentials,
                                format!(
                                    "account {} has usable {}-mode authentication",
                                    selected.as_str(),
                                    account.mode.spelling()
                                ),
                            )
                        } else {
                            // Shadowing is a warning on the existing row rather
                            // than a new check id: the credential is fine, and
                            // what the user needs to know is that something
                            // else will be used instead of it.
                            CheckResult::defect(
                                credentials,
                                warnings
                                    .iter()
                                    .map(|warning| warning.message())
                                    .collect::<Vec<_>>()
                                    .join("; "),
                                concat!(
                                    "remove the shadowing credential from the environment, ",
                                    "or accept that it is what the child will use"
                                )
                                .to_owned(),
                            )
                        }
                    }
                    _ => {
                        let mut hint = credentials
                            .hint(&[("account", selected.as_str())])
                            .unwrap_or_default();
                        let credential_link = [
                            context.paths().account_oauth_token(selected),
                            context.paths().account_auth_mode(selected),
                            context.paths().account_credentials(selected),
                        ]
                        .iter()
                        .any(|path| is_symlink(path));
                        if credential_link {
                            hint.push_str(concat!(
                                " Anything holding that link may have read this",
                                " account's credential. Treat it as exposed: run",
                                " `claude-session account login ",
                            ));
                            hint.push_str(selected.as_str());
                            hint.push_str(
                                "` for a fresh one, and revoke the old one at the provider.",
                            );
                        }
                        CheckResult::defect(
                            credentials,
                            format!(
                                concat!(
                                    "account {} is missing, unsafe, or has no usable ",
                                    "stored authentication"
                                ),
                                selected.as_str()
                            ),
                            hint,
                        )
                    }
                },
            };
            [registry_result, credentials_result]
        }
    }
}

fn inspect(context: &AppContext, name: Identifier) -> AccountFinding {
    let paths = context.paths();
    let directory = paths.account(&name);
    let metadata_path = paths.account_auth_mode(&name);
    let selected = context.account_selection().account() == Some(&name);
    let directory_safe = owned_directory(&directory);
    let metadata = read_metadata(context.paths().state(), &metadata_path).ok();
    let (mode, usable) = match metadata {
        Some(metadata) => {
            // `look` is an `lstat`, which does not follow the final component
            // but does follow every one above it. The login artifact sits one
            // level deeper than the account directory, so `config` has to be
            // checked in its own right or a `config` symlink would let a file
            // outside the private tree answer the usability question.
            let (artifact, intermediate_safe) = match metadata.mode {
                AuthMode::Login => (
                    paths.account_credentials(&name),
                    owned_directory(&paths.account_config(&name)),
                ),
                AuthMode::Token => (paths.account_oauth_token(&name), true),
            };
            (
                match metadata.mode {
                    AuthMode::Login => ReportMode::Login,
                    AuthMode::Token => ReportMode::Token,
                },
                directory_safe && intermediate_safe && owned_regular(&artifact),
            )
        }
        None => (ReportMode::Invalid, false),
    };
    AccountFinding {
        name,
        mode,
        usable,
        selected,
    }
}

// A wrong mode on a wrapper-managed directory is corrected rather than
// refused (xdg-storage.md#filesystem-security), so it cannot make an account
// unusable here without contradicting the launch, which corrects it and
// proceeds. Symlink, owner, and type remain genuine refusals.
fn owned_directory(path: &Path) -> bool {
    matches!(SystemFileSystem::look(path), Ok(Some(facts))
        if !facts.symlink && facts.directory && facts.uid == current_uid())
}

fn owned_regular(path: &Path) -> bool {
    matches!(SystemFileSystem::look(path), Ok(Some(facts))
        if !facts.symlink && facts.regular && facts.uid == current_uid())
}

fn is_symlink(path: &Path) -> bool {
    matches!(SystemFileSystem::look(path), Ok(Some(facts)) if facts.symlink)
}

fn current_uid() -> u32 {
    rustix::process::getuid().as_raw()
}

fn read_metadata(root: &Path, path: &Path) -> Result<AuthModeMetadata, AppError> {
    guard::validate(root, path, guard::Expected::PrivateFile)?;
    let mut handle = SystemFileSystem::open_private_file(path)
        .map_err(|error| path_error("authentication metadata is unavailable", path, &error))?;
    let mut bytes = Vec::new();
    handle
        .read_to_end(&mut bytes)
        .map_err(|error| path_error("authentication metadata could not be read", path, &error))?;
    serde_json::from_slice(&bytes).map_err(|error| {
        AppError::new(
            ErrorKind::DataFormat,
            Diagnostic::new(
                "authentication metadata is malformed",
                path.display().to_string(),
                error.to_string(),
                "run claude-session account login for this account",
            ),
        )
    })
}

pub(crate) struct LaunchAccount {
    pub(crate) mode: AuthMode,
}

pub(crate) fn validate_selected_launch(
    context: &AppContext,
) -> Result<Option<LaunchAccount>, AppError> {
    let Some(account) = context.session().account() else {
        return Ok(None);
    };
    guard::validate(
        context.paths().state(),
        &account.directory,
        guard::Expected::Directory,
    )?;
    guard::validate(
        context.paths().state(),
        &account.config,
        guard::Expected::Directory,
    )?;
    let metadata_path = context.paths().account_auth_mode(&account.id);
    let metadata = read_metadata(context.paths().state(), &metadata_path).map_err(|error| {
        match error.kind() {
            // Absent or unparsable mode metadata is semantic unusability the
            // account owner repairs by logging in again, so it is `Auth`. A
            // security defect or a genuine access failure keeps the kind its
            // own owner assigns (exit-codes.md), because `err.kind` is public
            // machine-readable API and selects the remediation.
            // `look` separates `Ok(None)` from `Err` deliberately, so only the
            // former is absence. A confirming probe that itself fails is a
            // second access failure, not evidence the document is missing.
            ErrorKind::DataFormat => auth_error(&error),
            ErrorKind::Io if matches!(SystemFileSystem::look(&metadata_path), Ok(None)) => {
                auth_message(
                    account.id.as_str(),
                    "the account has no recorded authentication mode",
                )
            }
            _ => error,
        }
    })?;
    match metadata.mode {
        AuthMode::Login if owned_regular(&context.paths().account_credentials(&account.id)) => {
            Ok(Some(LaunchAccount {
                mode: AuthMode::Login,
            }))
        }
        AuthMode::Login => Err(auth_message(
            account.id.as_str(),
            "the child-owned saved login is missing or unsafe",
        )),
        // Presence and safety only. The token is not read here and not
        // fingerprinted here: a launch injects a credential the child then
        // judges, and a consistency check at this point would invent a failure
        // the rotation order deliberately made survivable.
        AuthMode::Token if owned_regular(&context.paths().account_oauth_token(&account.id)) => {
            Ok(Some(LaunchAccount {
                mode: AuthMode::Token,
            }))
        }
        AuthMode::Token => Err(auth_message(
            account.id.as_str(),
            "the stored token is missing or unsafe",
        )),
    }
}

/// Reports every ambient mechanism that outranks a selected account.
///
/// Observation only. Each of these belongs to the user's environment, the
/// wrapper never strips one, and finding one changes nothing it stores — which
/// is why this returns warnings rather than an error, and why the same list
/// feeds the pre-launch line, `account status`, and `doctor`.
pub(crate) fn ambient_warnings(context: &AppContext) -> Vec<Warning> {
    let variables = context.environment().variables();
    AmbientCredential::ALL
        .into_iter()
        .filter(|credential| {
            crate::adapters::environment::value(variables, credential.spelling())
                .is_some_and(|value| !value.is_empty())
        })
        .map(Warning::Ambient)
        .collect()
}

/// Returns the warnings that apply to launching one account.
///
/// Token-over-login shadowing is added on top of the ambient set, because a
/// saved login inside a token account's configuration directory is real state
/// the user may believe is in use.
pub(crate) fn launch_warnings(
    context: &AppContext,
    account: &Identifier,
    mode: ReportMode,
) -> Vec<Warning> {
    let mut warnings = ambient_warnings(context);
    if mode == ReportMode::Token && owned_regular(&context.paths().account_credentials(account)) {
        warnings.push(Warning::TokenOverLogin);
    }
    warnings
}

pub(crate) fn enforce_version_floor(context: &AppContext, program: &Path) -> Result<(), AppError> {
    let invocation = ChildInvocation::new(
        program.to_path_buf(),
        vec!["--version".into()],
        super::child::subroutine_environment(context),
    );
    let output = context.adapters().process().run_captured(&invocation)?;
    let observed = match output.outcome {
        ChildOutcome::Exited(0) => ChildVersion::parse(&output.stdout),
        _ => None,
    };
    if observed.is_some_and(|version| version >= MINIMUM_CHILD_VERSION) {
        return Ok(());
    }
    let version = observed.map_or_else(|| "unavailable".into(), |value| value.to_string());
    let hint = Check::ChildVersionFloor
        .hint(&[
            ("version", &version),
            ("minimum", &MINIMUM_CHILD_VERSION.to_string()),
        ])
        .unwrap_or_default();
    Err(AppError::new(
        ErrorKind::Unavailable,
        Diagnostic::new(
            "the child version is too old for shared saved login",
            program.display().to_string(),
            observed.map_or_else(
                || "the child version could not be determined".into(),
                |value| format!("observed {value}; minimum is {MINIMUM_CHILD_VERSION}"),
            ),
            hint,
        ),
    ))
}

pub(crate) fn write_marker(context: &AppContext) -> Result<(), AppError> {
    let Some(account) = context.account_selection().account() else {
        return Ok(());
    };
    let marker = context.paths().last_account();
    let parent = marker.parent().ok_or_else(|| {
        AppError::new(
            ErrorKind::Internal,
            Diagnostic::new(
                "marker path has no parent",
                marker.display().to_string(),
                "the XDG path is incomplete",
                "report this invariant",
            ),
        )
    })?;
    guard::ensure_directory(context.paths().state(), parent)?;
    guard::validate(
        context.paths().state(),
        &marker,
        guard::Expected::PrivateFile,
    )?;
    atomic::write(&marker, account.as_str().as_bytes(), 0o600)
}

/// Commits a completed native login's mode metadata.
///
/// Under the credential lock, though login mode mints no wrapper-owned secret.
/// The scope guards `auth-mode.json` as well as the token, and removal treats
/// unlinking that file as its own commit — so a login-mode write outside the
/// lock could land inside a tree a concurrent `account remove` had already
/// committed to destroying, leaving metadata describing an account that is
/// being deleted around it.
pub(crate) fn write_login_metadata(
    context: &AppContext,
    account: &Identifier,
) -> Result<AuthModeMetadata, AppError> {
    let metadata = AuthModeMetadata {
        mode: AuthMode::Login,
        recorded_at: RecordedAt::new_unchecked(rfc3339_utc(context.adapters().clock().now())),
        fingerprint: None,
    };
    let _lock = hold(context, account)?;
    write_metadata(context, account, &metadata)?;
    Ok(metadata)
}

/// Acquires one account's credential lock at the standard deadline.
pub(crate) fn hold(
    context: &AppContext,
    account: &Identifier,
) -> Result<lock::CredentialLock, AppError> {
    lock::acquire(
        context.paths().state(),
        &context.paths().account_credentials_lock(account),
        lock::Deadline::STANDARD,
    )
}

/// Encodes and atomically writes one account's mode metadata.
///
/// In token mode this rename is the commit, so the caller holds the credential
/// lock across it and writes the token first. Here it is only a write.
pub(crate) fn write_metadata(
    context: &AppContext,
    account: &Identifier,
    metadata: &AuthModeMetadata,
) -> Result<(), AppError> {
    let mut bytes = serde_json::to_vec(metadata).map_err(|error| {
        AppError::new(
            ErrorKind::Internal,
            Diagnostic::new(
                "authentication metadata could not be encoded",
                account.as_str(),
                error.to_string(),
                "report this invariant",
            ),
        )
    })?;
    bytes.push(b'\n');
    let path = context.paths().account_auth_mode(account);
    guard::validate(context.paths().state(), &path, guard::Expected::PrivateFile)?;
    atomic::write(&path, &bytes, 0o600)
}

/// Reports whether the child left a safe saved login this run may commit.
///
/// The account directory and its `config` are revalidated here rather than
/// relied on from `prepare_login`: an interactive login can take minutes, and
/// `xdg-storage.md#how-a-path-is-validated` requires the walk to run against
/// the state the operation will meet rather than the state it met earlier. The
/// probe on the credential itself is an `lstat`, which follows every component
/// above the leaf, so a `config` replaced by a link while the child ran would
/// otherwise let a file outside the account tree commit the login.
pub(crate) fn credentials_committable(
    context: &AppContext,
    account: &Identifier,
) -> Result<bool, AppError> {
    let state = context.paths().state();
    guard::validate(
        state,
        &context.paths().account(account),
        guard::Expected::Directory,
    )?;
    guard::validate(
        state,
        &context.paths().account_config(account),
        guard::Expected::Directory,
    )?;
    Ok(owned_regular(&context.paths().account_credentials(account)))
}

/// Reports whether the last-used marker names this account.
///
/// Read directly rather than taken from the resolved selection, because the
/// selection may have come from a flag or from configuration and this question
/// is about the marker file alone. A marker that cannot be read answers `false`,
/// so a removal never clears a selection it could not confirm.
pub(super) fn marker_names(context: &AppContext, account: &Identifier) -> bool {
    let marker = context.paths().last_account();
    SystemFileSystem::open_private_file(&marker)
        .and_then(|mut handle| {
            let mut bytes = Vec::new();
            handle.read_to_end(&mut bytes)?;
            Ok(bytes)
        })
        .is_ok_and(|bytes| bytes == account.as_str().as_bytes())
}

pub(crate) fn prepare_login(context: &AppContext, account: &Identifier) -> Result<(), AppError> {
    guard::ensure_directory(context.paths().state(), &context.paths().account(account))?;
    guard::ensure_directory(
        context.paths().state(),
        &context.paths().account_config(account),
    )
}

/// Reports whether an account has committed mode metadata.
///
/// The metadata rename is what makes an account an account, so its presence is
/// the one test for "somebody finished a login here" that does not depend on
/// which run is asking.
pub(crate) fn committed(context: &AppContext, account: &Identifier) -> bool {
    owned_regular(&context.paths().account_auth_mode(account))
}

pub(crate) fn remove_incomplete(path: &Path) -> Result<(), AppError> {
    remove_tree(path)
        .map_err(|error| path_error("incomplete account could not be removed", path, &error))
}

/// Removes a directory and everything beneath it, in no particular order.
///
/// Correct for the two cases that have no commit point: cleaning up a tree this
/// run just created, and clearing one sub-tree whose internal order nothing
/// depends on. Ordered removal of an account is a different contract and lives
/// in [`remove`], because there the first unlink is what makes the removal
/// irreversible.
pub(super) fn remove_tree(path: &Path) -> std::io::Result<()> {
    let Some(facts) = SystemFileSystem::look(path)? else {
        return Ok(());
    };
    if facts.symlink || !facts.directory {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "cleanup root is not a directory",
        ));
    }
    for entry in fs::read_dir(path)? {
        let child = entry?.path();
        let facts = SystemFileSystem::look(&child)?.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "cleanup entry disappeared")
        })?;
        if facts.directory && !facts.symlink {
            remove_tree(&child)?;
        } else {
            fs::remove_file(&child)?;
        }
    }
    fs::remove_dir(path)
}

fn auth_error(error: &AppError) -> AppError {
    auth_message(&error.diagnostic().where_, &error.diagnostic().why)
}
fn auth_message(where_: &str, why: &str) -> AppError {
    AppError::new(
        ErrorKind::Auth,
        Diagnostic::new(
            "selected account authentication is unusable",
            where_,
            why,
            "run claude-session account login for this account",
        ),
    )
}
fn path_error(what: &str, path: &Path, error: &std::io::Error) -> AppError {
    let kind = if error.kind() == std::io::ErrorKind::PermissionDenied {
        ErrorKind::Permission
    } else {
        ErrorKind::Io
    };
    AppError::new(
        kind,
        Diagnostic::new(
            what,
            path.display().to_string(),
            error.to_string(),
            "check the path and its ownership",
        ),
    )
}
