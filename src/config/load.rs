//! Five-layer decoding with per-key winning provenance.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    adapters::{environment::Environment, filesystem::SystemFileSystem},
    commands::dispatch::Globals,
    domain::{
        config::{ResolvedConfig, Source},
        identifier::Identifier,
        paths::XdgPaths,
    },
    error::ConfigError,
};
use figment::{
    Figment,
    providers::{Format, Toml},
};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    child_bin: Option<PathBuf>,
    default_account: Option<String>,
    default_profile: Option<String>,
}

/// Resolves defaults, files, environment, then CLI values.
pub(crate) fn resolve(
    environment: &impl Environment,
    paths: &XdgPaths,
    globals: &Globals,
) -> Result<ResolvedConfig, ConfigError> {
    let mut resolved = ResolvedConfig::defaults();
    let user = globals
        .config
        .clone()
        .unwrap_or_else(|| paths.config().join("config.toml"));
    apply_file(&mut resolved, &user, Source::User, false)?;
    if let Some(project) = super::project::discover(environment.current_dir()) {
        apply_file(&mut resolved, &project, Source::Project, true)?;
    }
    apply_environment(&mut resolved, environment.variables())?;
    if let Some(path) = globals.config.as_ref() {
        let _ = path;
    }
    if let Some(account) = globals.account.clone() {
        resolved.account_mut().set(account, Source::Cli);
    }
    if let Some(profile) = globals.profile.clone() {
        resolved.profile_mut().set(profile, Source::Cli);
    }
    Ok(resolved)
}

fn apply_file(
    resolved: &mut ResolvedConfig,
    path: &Path,
    source: Source,
    project: bool,
) -> Result<(), ConfigError> {
    let bytes = match SystemFileSystem::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(ConfigError::Read {
                path: path.to_path_buf(),
                source: error,
            });
        }
    };
    let text = std::str::from_utf8(&bytes).map_err(|error| ConfigError::Decode {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let _: toml::Value = toml::from_str(text).map_err(|error| ConfigError::Decode {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let file: FileConfig = Figment::new()
        .merge(Toml::string(text))
        .extract()
        .map_err(|error| ConfigError::Decode {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    if project && (file.child_bin.is_some() || file.default_account.is_some()) {
        let key = if file.child_bin.is_some() {
            "child_bin"
        } else {
            "default_account"
        };
        return Err(ConfigError::Decode {
            path: path.to_path_buf(),
            message: format!("key `{key}` is not allowed in a project file"),
        });
    }
    if let Some(value) = file.child_bin {
        resolved.child_bin_mut().set(value, source);
    }
    if let Some(value) = file.default_account {
        resolved
            .account_mut()
            .set(identifier("default_account", &value, path)?, source);
    }
    if let Some(value) = file.default_profile {
        resolved
            .profile_mut()
            .set(identifier("default_profile", &value, path)?, source);
    }
    Ok(())
}

fn apply_environment(
    resolved: &mut ResolvedConfig,
    environment: &[(OsString, OsString)],
) -> Result<(), ConfigError> {
    for (key, value) in environment {
        match key.to_str() {
            Some("CLAUDE_SESSION_CHILD_BIN") => resolved
                .child_bin_mut()
                .set(PathBuf::from(value), Source::Environment),
            Some("CLAUDE_SESSION_DEFAULT_ACCOUNT") => resolved.account_mut().set(
                environment_identifier("default_account", value)?,
                Source::Environment,
            ),
            Some("CLAUDE_SESSION_DEFAULT_PROFILE") => resolved.profile_mut().set(
                environment_identifier("default_profile", value)?,
                Source::Environment,
            ),
            _ => {}
        }
    }
    Ok(())
}

fn identifier(key: &'static str, value: &str, path: &Path) -> Result<Identifier, ConfigError> {
    Identifier::from_str(value).map_err(|_| ConfigError::Identifier {
        key,
        origin: path.display().to_string(),
        value: value.to_owned(),
    })
}

fn environment_identifier(key: &'static str, value: &OsStr) -> Result<Identifier, ConfigError> {
    // A non-UTF-8 value is not an identifier-grammar failure, so it keeps the
    // `Config` typing: there is no value to quote back and nothing about the
    // grammar to correct.
    let text = value.to_str().ok_or_else(|| ConfigError::Value {
        key,
        origin: "environment (non-UTF-8)".into(),
    })?;
    Identifier::from_str(text).map_err(|_| ConfigError::Identifier {
        key,
        origin: environment_spelling(key).to_owned(),
        value: text.to_owned(),
    })
}

/// Names the environment variable a key is read from, so a diagnostic points at
/// the thing the user would unset rather than at the file spelling.
const fn environment_spelling(key: &str) -> &'static str {
    match key.as_bytes() {
        b"default_account" => "CLAUDE_SESSION_DEFAULT_ACCOUNT",
        _ => "CLAUDE_SESSION_DEFAULT_PROFILE",
    }
}
