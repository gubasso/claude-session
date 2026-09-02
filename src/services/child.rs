//! Terminal child resolution, recursion guards, and invocation preparation.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    adapters::{
        environment::{self, Environment},
        filesystem::FileSystem,
    },
    context::AppContext,
    domain::{account::AuthMode, child::ChildInvocation, config::ResolvedConfig},
    error::{AppError, Diagnostic},
};

/// Resolves the terminal child ladder and applies both recursion guards.
///
/// The filesystem arrives as a port so the ladder can be exercised against a
/// fake: every branch below turns on metadata, permission, and identity, none
/// of which a test should have to lay down on disk to reach.
pub(crate) fn resolve<F: FileSystem>(
    filesystem: &F,
    environment: &[(OsString, OsString)],
    current_exe: &Path,
    config: &ResolvedConfig,
) -> Result<PathBuf, AppError> {
    if environment::value(environment, "CLAUDE_SESSION_REENTRY").is_some_and(|value| value == "1") {
        return Err(AppError::new(
            crate::error::ErrorKind::ChildRecursion,
            Diagnostic::new(
                "recursive wrapper invocation refused",
                "CLAUDE_SESSION_REENTRY",
                "the child environment contains the wrapper marker",
                "remove wrapper recursion from child_bin or PATH",
            ),
        ));
    }
    let path = if let Some(override_path) = config.child_bin() {
        validate(filesystem, override_path, Candidate::Configured)?
    } else {
        search_path(filesystem, environment)?
    };
    let current = filesystem.identity(current_exe).map_err(|error| {
        AppError::new(
            crate::error::ErrorKind::OsError,
            Diagnostic::new(
                "current executable identity is unavailable",
                current_exe.display().to_string(),
                error.to_string(),
                "retry from an installed executable",
            ),
        )
    })?;
    let child = filesystem.identity(&path).map_err(|error| {
        AppError::child_not_found(path.display().to_string(), error.to_string())
    })?;
    if current == child {
        return Err(AppError::new(
            crate::error::ErrorKind::ChildRecursion,
            Diagnostic::new(
                "resolved child is this wrapper",
                path.display().to_string(),
                "device and inode match the current executable",
                "configure the real claude executable",
            ),
        ));
    }
    tracing::info!(
        op = "resolve_child",
        status = "ok",
        path = %path.display(),
        source = ?config.child_bin_source(),
        "this run will use the claude at {}",
        path.display()
    );
    Ok(path)
}

/// Builds the scrubbed child environment and untouched argument suffix.
///
/// This is the bare form, and the one a subroutine question uses: it adds no
/// composed settings and selects no account, because a verb asking the child
/// something is the child's caller rather than its passthrough. See [ADR-0068].
///
/// [ADR-0068]: ../../docs/decisions/ADR-0068-spawn-the-child-as-a-subroutine.md
pub(crate) fn invocation(
    context: &AppContext,
    arguments: Vec<OsString>,
) -> Result<ChildInvocation, AppError> {
    Ok(ChildInvocation::new(
        program(context)?,
        arguments,
        scrubbed(context),
    ))
}

/// Resolves the child from the context, which is the launch sequence's first step.
///
/// It is a step of its own, and the caller's rather than the builder's, because
/// the sequence puts it before any storage work: a child that cannot be resolved
/// — or a recursion the guard refuses — must fail before the wrapper creates an
/// account directory or materialises a composed entry. See [process runtime].
///
/// [process runtime]: ../../docs/reference/process-runtime.md#the-exec
pub(crate) fn program(context: &AppContext) -> Result<PathBuf, AppError> {
    resolve(
        &context.adapters().filesystem(),
        context.environment().variables(),
        context.environment().current_exe(),
        context.config(),
    )
}

/// Builds the invocation a passthrough launch replaces itself with.
///
/// One argument addition over [`invocation`], and only this one: the composed
/// settings pair ahead of an untouched user suffix. Alongside it the account
/// configuration directory the child reads its own state from, and the seed
/// tree it reads its plugins from when the user keeps one ([ADR-0117]). All
/// stay OS strings, because the paths derive from the XDG bases, whose bytes
/// are arbitrary. The pair and its precedence belong to [ADR-0028].
///
/// [ADR-0117]: ../../docs/decisions/ADR-0117-supply-plugins-from-a-read-only-seed.md
///
/// The child arrives already resolved, by [`program`], because resolution is an
/// earlier step of the sequence than the account and profile inputs this reads.
///
/// [ADR-0028]: ../../docs/decisions/ADR-0028-pass-composed-settings-with-the-native-flag.md
/// In token mode a third addition applies: the stored token, in the variable
/// the child documents for it, and beside it the plan the account declared, in
/// the variable the child reads a plan from. The token is read here rather than
/// earlier because
/// `xdg-storage.md#how-a-path-is-validated` requires the walk to run against
/// the state the operation will meet, and this is that operation. An ambient
/// value of either name is dropped rather than merged, so a selected token
/// account launches with its own token and with its own declaration or none.
pub(crate) fn launch(
    context: &AppContext,
    program: PathBuf,
    settings: Option<&Path>,
    arguments: Vec<OsString>,
    auth: Option<&crate::services::account::LaunchAccount>,
    session: &Path,
    plugin_seed: Option<&Path>,
) -> Result<ChildInvocation, AppError> {
    let mode = auth.map(|auth| auth.mode);
    let mut vector = Vec::with_capacity(arguments.len() + 2);
    if let Some(path) = settings {
        vector.push(OsString::from("--settings"));
        vector.push(path.as_os_str().to_owned());
    }
    vector.extend(arguments);
    let mut environment = scrubbed(context);
    if let Some(account) = context.session().account() {
        // The configuration directory is this terminal's, and the credential
        // store is the account's. Splitting them is the whole design: three of
        // the files the child writes into the first are keyed by nothing and
        // interleave between panes, while the saved login in the second must
        // stay one file for the child's own refresh coordination to mean
        // anything ([ADR-0102], [ADR-0104]).
        //
        // Both are absolute and already resolved. The child compares the store
        // value after Unicode normalization only — no canonicalization and no
        // tilde expansion — so a value it has to interpret would select a
        // different store than the wrapper prepared.
        //
        // [ADR-0102]: ../../docs/decisions/ADR-0102-key-child-state-by-terminal.md
        // [ADR-0104]: ../../docs/decisions/ADR-0104-share-one-credential-store.md
        environment.push(("CLAUDE_CONFIG_DIR".into(), session.as_os_str().to_owned()));
        environment.retain(|(key, _)| key != "CLAUDE_SECURESTORAGE_CONFIG_DIR");
        environment.push((
            "CLAUDE_SECURESTORAGE_CONFIG_DIR".into(),
            account.config.as_os_str().to_owned(),
        ));
        // Beside the configuration directory, because it describes what that
        // directory can reach: the child locates a seeded marketplace by
        // probing this tree rather than by trusting the paths recorded inside
        // it, which is what lets one read-only tree serve every session
        // ([ADR-0117]).
        //
        // Replaced rather than merged, for the reason the credential store is:
        // an ambient value points at whatever tree the surrounding environment
        // was built for, and a session that inherited it would reach plugins
        // the wrapper did not select. Absent when the user keeps no seed, which
        // leaves the child exactly as it behaves without this feature.
        //
        // [ADR-0117]: ../../docs/decisions/ADR-0117-supply-plugins-from-a-read-only-seed.md
        environment.retain(|(key, _)| key != "CLAUDE_CODE_PLUGIN_SEED_DIR");
        if let Some(seed) = plugin_seed {
            environment.push((
                "CLAUDE_CODE_PLUGIN_SEED_DIR".into(),
                seed.as_os_str().to_owned(),
            ));
        }
        if mode == Some(AuthMode::Token) {
            let token = crate::services::account::token::read_stored(context, &account.id)?;
            environment.retain(|(key, _)| key != "CLAUDE_CODE_OAUTH_TOKEN");
            environment.push((
                "CLAUDE_CODE_OAUTH_TOKEN".into(),
                OsString::from(String::from_utf8_lossy(token.expose()).into_owned()),
            ));
            // Beside the token, because it describes the token: the child reads
            // its plan from the credential it saved for itself, and an injected
            // token is not that credential. Only when one was declared — an
            // absent variable leaves the child on its own fallback, which is
            // what it does today, while an empty one would be a declaration of
            // nothing ([ADR-0099]).
            //
            // The removal is unconditional, for the same reason the token's is:
            // an ambient value describes whatever credential the environment was
            // built for, not the one being injected here. Leaving it in place
            // when the account declared nothing would let it stand in for a
            // declaration the account does not have, contradicting the warning
            // the launch has already written.
            //
            // [ADR-0099]: ../../docs/decisions/ADR-0099-declare-the-plan-a-token-cannot-carry.md
            environment.retain(|(key, _)| key != "CLAUDE_CODE_SUBSCRIPTION_TYPE");
            if let Some(plan) = auth.and_then(|auth| auth.plan.as_ref()) {
                environment.push((
                    "CLAUDE_CODE_SUBSCRIPTION_TYPE".into(),
                    OsString::from(plan.as_str()),
                ));
            }
        }
    }
    Ok(ChildInvocation::new(program, vector, environment))
}

/// Snapshots the environment, removes the wrapper's namespace, restores the marker.
///
/// The order is the contract: setting the marker before the scrub would delete
/// it again. See [ADR-0057].
///
/// [ADR-0057]:
///     ../../docs/decisions/ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md
fn scrubbed(context: &AppContext) -> Vec<(OsString, OsString)> {
    let mut environment: Vec<_> = context
        .environment()
        .variables()
        .iter()
        .filter(|(key, _)| !key.as_encoded_bytes().starts_with(b"CLAUDE_SESSION_"))
        .cloned()
        .collect();
    environment.push(("CLAUDE_SESSION_REENTRY".into(), "1".into()));
    environment
}

/// Returns the bare environment used by child inspection subroutines.
pub(crate) fn subroutine_environment(context: &AppContext) -> Vec<(OsString, OsString)> {
    scrubbed(context)
}

/// Walks `PATH` once, remembering the first refusal.
///
/// A candidate that exists but cannot be executed decides 126 only if nothing
/// later on the path can be executed at all, so the walk continues past it. It
/// is one walk rather than two because a second, independent implementation of
/// the same ladder can disagree with the first about which candidate wins.
fn search_path<F: FileSystem>(
    filesystem: &F,
    environment: &[(OsString, OsString)],
) -> Result<PathBuf, AppError> {
    let Some(path) = environment::value(environment, "PATH") else {
        return Err(AppError::child_not_found("PATH", "PATH is unset"));
    };
    let mut refusal = None;
    for directory in std::env::split_paths(path).filter(|entry| !entry.as_os_str().is_empty()) {
        let candidate = directory.join("claude");
        match validate(filesystem, &candidate, Candidate::Searched) {
            Ok(path) => return Ok(path),
            // An absent candidate says nothing: it is simply the next entry.
            // Anything else — a refusal or a filesystem fault — is the most
            // informative thing the walk saw, and outlives it.
            Err(error) if error.kind() == crate::error::ErrorKind::ChildNotFound => {}
            Err(error) => {
                refusal.get_or_insert(error);
            }
        }
    }
    refusal.map_or_else(
        || {
            Err(AppError::child_not_found(
                "PATH",
                "no executable claude was found",
            ))
        },
        Err,
    )
}

/// Where a candidate came from, which decides how its rejection reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Candidate {
    /// Named by `child_bin`, so a bad value is the user's configuration.
    Configured,
    /// Found on `PATH`, so a bad value is just the next entry to try.
    Searched,
}

fn validate<F: FileSystem>(
    filesystem: &F,
    path: &Path,
    candidate: Candidate,
) -> Result<PathBuf, AppError> {
    if !path.is_absolute() {
        return if candidate == Candidate::Configured {
            Err(AppError::new(
                crate::error::ErrorKind::Config,
                Diagnostic::new(
                    "child_bin must be absolute",
                    path.display().to_string(),
                    "configured overrides are absolute paths",
                    "use an absolute child_bin",
                ),
            ))
        } else {
            Err(AppError::child_not_found(
                path.display().to_string(),
                "candidate is not absolute",
            ))
        };
    }
    let facts = filesystem.describe(path).map_err(|error| {
        AppError::child_not_found(path.display().to_string(), error.to_string())
    })?;
    if !facts.regular {
        return Err(AppError::child_not_executable(
            path.display().to_string(),
            "candidate is not a regular file",
        ));
    }
    if facts.mode & 0o111 == 0 {
        return Err(AppError::child_not_executable(
            path.display().to_string(),
            "execute access was denied",
        ));
    }
    match filesystem.executable(path) {
        Ok(true) => {}
        Ok(false) => {
            return Err(AppError::child_not_executable(
                path.display().to_string(),
                "execute access was denied",
            ));
        }
        // A failed access check is not a denial. Reporting it as one would tell
        // the user to fix a mode when the filesystem itself is the problem.
        Err(error) => {
            return Err(AppError::new(
                crate::error::ErrorKind::OsError,
                Diagnostic::new(
                    "executable access could not be checked",
                    path.display().to_string(),
                    error.to_string(),
                    "check the filesystem holding the candidate",
                ),
            ));
        }
    }
    filesystem
        .canonicalize(path)
        .map_err(|error| AppError::child_not_found(path.display().to_string(), error.to_string()))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A filesystem that exists only in memory.
    ///
    /// The ladder's branches turn on metadata, mode, access, and identity. A
    /// real tree can express the first three and cannot express a failing
    /// `access` at all without a hostile mount.
    #[derive(Default)]
    struct FakeFs {
        files: HashMap<PathBuf, (u32, bool)>,
        access_error: bool,
        identities: HashMap<PathBuf, (u64, u64)>,
    }

    impl FakeFs {
        fn with_file(mut self, path: &str, mode: u32, regular: bool) -> Self {
            self.files.insert(PathBuf::from(path), (mode, regular));
            self
        }
        fn with_identity(mut self, path: &str, identity: (u64, u64)) -> Self {
            self.identities.insert(PathBuf::from(path), identity);
            self
        }
        fn missing(path: &Path) -> std::io::Error {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no such file {}", path.display()),
            )
        }
    }

    impl FileSystem for FakeFs {
        fn describe(&self, path: &Path) -> std::io::Result<crate::adapters::filesystem::FileFacts> {
            self.files.get(path).map_or_else(
                || Err(Self::missing(path)),
                |(mode, regular)| {
                    Ok(crate::adapters::filesystem::FileFacts {
                        regular: *regular,
                        mode: *mode,
                    })
                },
            )
        }
        fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
            if self.files.contains_key(path) {
                Ok(path.to_path_buf())
            } else {
                Err(Self::missing(path))
            }
        }
        fn executable(&self, path: &Path) -> std::io::Result<bool> {
            if self.access_error {
                return Err(std::io::Error::from_raw_os_error(5));
            }
            Ok(self
                .files
                .get(path)
                .is_some_and(|(mode, _)| mode & 0o111 != 0))
        }
        fn identity(&self, path: &Path) -> std::io::Result<(u64, u64)> {
            self.identities
                .get(path)
                .copied()
                .ok_or_else(|| Self::missing(path))
        }
    }

    fn env(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
        pairs
            .iter()
            .map(|(key, value)| (OsString::from(*key), OsString::from(*value)))
            .collect()
    }

    #[test]
    fn the_reentry_marker_refuses_before_any_filesystem_access() {
        let error = resolve(
            &FakeFs::default(),
            &env(&[("CLAUDE_SESSION_REENTRY", "1")]),
            Path::new("/usr/bin/claude-session"),
            &ResolvedConfig::defaults(),
        )
        .expect_err("recursion refused");
        assert_eq!(error.kind(), crate::error::ErrorKind::ChildRecursion);
    }

    #[test]
    fn an_unset_path_is_not_found_rather_than_a_panic() {
        let error = resolve(
            &FakeFs::default(),
            &env(&[]),
            Path::new("/usr/bin/claude-session"),
            &ResolvedConfig::defaults(),
        )
        .expect_err("PATH unset");
        assert_eq!(error.kind(), crate::error::ErrorKind::ChildNotFound);
    }

    #[test]
    fn a_relative_configured_override_is_a_configuration_error() {
        let mut config = ResolvedConfig::defaults();
        config
            .child_bin_mut()
            .set(PathBuf::from("claude"), crate::domain::config::Source::User);
        let error = resolve(
            &FakeFs::default(),
            &env(&[]),
            Path::new("/usr/bin/claude-session"),
            &config,
        )
        .expect_err("relative override");
        assert_eq!(error.kind(), crate::error::ErrorKind::Config);
    }

    /// The ladder used to stop at the first candidate a permission check
    /// rejected, so a later executable entry was never reached.
    #[test]
    fn a_refused_candidate_does_not_end_the_walk() {
        let filesystem = FakeFs::default()
            .with_file("/a/claude", 0o644, true)
            .with_file("/b/claude", 0o755, true)
            .with_identity("/b/claude", (1, 2))
            .with_identity("/usr/bin/claude-session", (1, 3));
        let found = search_path(&filesystem, &env(&[("PATH", "/a:/b")]));
        assert_eq!(found.expect("later entry wins"), PathBuf::from("/b/claude"));
    }

    /// With nothing executable anywhere, the refusal decides the code.
    #[test]
    fn a_refusal_decides_the_code_when_nothing_later_is_executable() {
        let filesystem = FakeFs::default().with_file("/a/claude", 0o644, true);
        let error = search_path(&filesystem, &env(&[("PATH", "/a")])).expect_err("not executable");
        assert_eq!(error.kind(), crate::error::ErrorKind::ChildNotExecutable);
    }

    /// A failing `access` call is a filesystem fault, not a denial. Reporting it
    /// as 126 would tell the user to repair a mode that is already correct.
    #[test]
    fn an_access_failure_is_an_os_error_not_a_denial() {
        let mut filesystem = FakeFs::default().with_file("/a/claude", 0o755, true);
        filesystem.access_error = true;
        let error = search_path(&filesystem, &env(&[("PATH", "/a")])).expect_err("access failed");
        assert_eq!(error.kind(), crate::error::ErrorKind::OsError);
    }

    #[test]
    fn a_child_sharing_the_wrapper_identity_is_refused() {
        let filesystem = FakeFs::default()
            .with_file("/a/claude", 0o755, true)
            .with_identity("/a/claude", (1, 2))
            .with_identity("/usr/bin/claude-session", (1, 2));
        let error = resolve(
            &filesystem,
            &env(&[("PATH", "/a")]),
            Path::new("/usr/bin/claude-session"),
            &ResolvedConfig::defaults(),
        )
        .expect_err("self recursion");
        assert_eq!(error.kind(), crate::error::ErrorKind::ChildRecursion);
    }
}
