//! Five-layer decoding with per-key winning provenance.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    adapters::{environment::Environment, filesystem::SystemFileSystem},
    commands::dispatch::ConfigOverrides,
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

/// One file layer that was looked at, and whether it was there.
///
/// `config` has to report which files were consulted and which existed
/// (`configuration.md#commands`), and "a missing file is not an error" makes
/// that fact unrecoverable from the resolved value alone.
#[derive(Clone, Debug)]
pub(crate) struct ConsultedFile {
    /// The path the layer looked at.
    pub(crate) path: PathBuf,
    /// The layer it would have supplied.
    pub(crate) source: Source,
    /// Whether the file was there.
    pub(crate) existed: bool,
}

/// The resolved configuration together with the files it came from.
#[derive(Clone, Debug)]
pub(crate) struct Resolution {
    /// The value precedence produced.
    pub(crate) config: ResolvedConfig,
    /// Every file layer with a candidate, in precedence order.
    pub(crate) consulted: Vec<ConsultedFile>,
}

/// Resolves defaults, files, environment, then CLI values.
///
/// A layer with no candidate contributes no row: the environment and the
/// command line are not files, and outside a repository there is no project
/// layer at all.
pub(crate) fn resolve(
    environment: &impl Environment,
    paths: &XdgPaths,
    overrides: &ConfigOverrides,
) -> Result<Resolution, ConfigError> {
    let mut resolved = ResolvedConfig::defaults();
    let mut consulted = Vec::new();
    let user = overrides
        .config
        .clone()
        .unwrap_or_else(|| paths.config().join("config.toml"));
    let existed = apply_file(&mut resolved, &user, Source::User, false)?;
    consulted.push(ConsultedFile {
        path: user,
        source: Source::User,
        existed,
    });
    if let Some(project) = super::project::discover(environment.current_dir()) {
        let existed = apply_file(&mut resolved, &project, Source::Project, true)?;
        consulted.push(ConsultedFile {
            path: project,
            source: Source::Project,
            existed,
        });
    }
    apply_environment(&mut resolved, environment.variables())?;
    if let Some(account) = overrides.account.clone() {
        resolved.account_mut().set(account, Source::Cli);
    }
    if let Some(profile) = overrides.profile.clone() {
        resolved.profile_mut().set(profile, Source::Cli);
    }
    Ok(Resolution {
        config: resolved,
        consulted,
    })
}

/// Applies one file layer, reporting whether the file was there.
fn apply_file(
    resolved: &mut ResolvedConfig,
    path: &Path,
    source: Source,
    project: bool,
) -> Result<bool, ConfigError> {
    let bytes = match SystemFileSystem::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
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
    Ok(true)
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
