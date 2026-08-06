//! Terminal child resolution, recursion guards, and invocation preparation.

use std::{
    ffi::{OsStr, OsString},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use crate::{
    adapters::{environment::Environment, filesystem::FileSystem},
    context::AppContext,
    domain::child::ChildInvocation,
    error::{AppError, Diagnostic},
};

/// Resolves the terminal child ladder and applies both recursion guards.
pub(crate) fn resolve(context: &AppContext) -> Result<PathBuf, AppError> {
    let _filesystem = context.adapters().filesystem();
    if environment_value(
        context.environment().variables(),
        OsStr::new("CLAUDE_SESSION_REENTRY"),
    )
    .is_some_and(|value| value == "1")
    {
        return Err(AppError::ChildRecursion(Diagnostic::new(
            "recursive wrapper invocation refused",
            "CLAUDE_SESSION_REENTRY",
            "the child environment contains the wrapper marker",
            "remove wrapper recursion from child_bin or PATH",
        )));
    }
    let path = if let Some(override_path) = context.config().child_bin() {
        validate(override_path, true)?
    } else {
        search_path(context.environment().variables())?
    };
    let current = FileSystem::identity(context.environment().current_exe()).map_err(|error| {
        AppError::OsError(Diagnostic::new(
            "current executable identity is unavailable",
            context.environment().current_exe().display().to_string(),
            error.to_string(),
            "retry from an installed executable",
        ))
    })?;
    let child = FileSystem::identity(&path).map_err(|error| {
        AppError::child_not_found(path.display().to_string(), error.to_string())
    })?;
    if current == child {
        return Err(AppError::ChildRecursion(Diagnostic::new(
            "resolved child is this wrapper",
            path.display().to_string(),
            "device and inode match the current executable",
            "configure the real claude executable",
        )));
    }
    tracing::info!(
        op = "resolve_child",
        status = "ok",
        path = %path.display(),
        source = ?context.config().child_bin_source(),
        "resolved child"
    );
    Ok(path)
}

/// Builds the scrubbed child environment and untouched argument suffix.
pub(crate) fn invocation(
    context: &AppContext,
    arguments: Vec<OsString>,
) -> Result<ChildInvocation, AppError> {
    let program = resolve(context)?;
    let mut environment: Vec<_> = context
        .environment()
        .variables()
        .iter()
        .filter(|(key, _)| !key.as_encoded_bytes().starts_with(b"CLAUDE_SESSION_"))
        .cloned()
        .collect();
    environment.push(("CLAUDE_SESSION_REENTRY".into(), "1".into()));
    Ok(ChildInvocation::new(program, arguments, environment))
}

fn search_path(environment: &[(OsString, OsString)]) -> Result<PathBuf, AppError> {
    let Some(path) = environment_value(environment, OsStr::new("PATH")) else {
        return Err(AppError::child_not_found("PATH", "PATH is unset"));
    };
    let filtered = std::env::join_paths(
        std::env::split_paths(path).filter(|entry| !entry.as_os_str().is_empty()),
    )
    .unwrap_or_default();
    if let Ok(found) = which::which_in("claude", Some(filtered), Path::new("/")) {
        return validate(&found, false);
    }
    let mut permission = None;
    for directory in std::env::split_paths(path).filter(|entry| !entry.as_os_str().is_empty()) {
        let candidate = directory.join("claude");
        match validate(&candidate, false) {
            Ok(path) => return Ok(path),
            Err(error) if error.kind() == crate::error::ErrorKind::ChildNotExecutable => {
                permission = Some(error);
            }
            Err(_) => {}
        }
    }
    permission.map_or_else(
        || {
            Err(AppError::child_not_found(
                "PATH",
                "no executable claude was found",
            ))
        },
        Err,
    )
}

fn validate(path: &Path, terminal: bool) -> Result<PathBuf, AppError> {
    if !path.is_absolute() {
        return if terminal {
            Err(AppError::Config(Diagnostic::new(
                "child_bin must be absolute",
                path.display().to_string(),
                "configured overrides are absolute paths",
                "use an absolute child_bin",
            )))
        } else {
            Err(AppError::child_not_found(
                path.display().to_string(),
                "candidate is not absolute",
            ))
        };
    }
    let metadata = FileSystem::metadata(path).map_err(|error| {
        AppError::child_not_found(path.display().to_string(), error.to_string())
    })?;
    if !metadata.is_file() {
        return Err(AppError::child_not_executable(
            path.display().to_string(),
            "candidate is not a regular file",
        ));
    }
    if metadata.permissions().mode() & 0o111 == 0 || !FileSystem::executable(path).unwrap_or(false)
    {
        return Err(AppError::child_not_executable(
            path.display().to_string(),
            "execute access was denied",
        ));
    }
    FileSystem::canonicalize(path)
        .map_err(|error| AppError::child_not_found(path.display().to_string(), error.to_string()))
}

fn environment_value<'a>(
    environment: &'a [(OsString, OsString)],
    key: &OsStr,
) -> Option<&'a OsStr> {
    environment
        .iter()
        .find(|(candidate, _)| candidate == key)
        .map(|(_, value)| value.as_os_str())
}
