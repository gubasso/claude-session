//! Five-layer decoding with per-key winning provenance.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    str::FromStr,
};

use super::schema::FileConfig;
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
    // A repository may not answer a question about whether to trust itself,
    // which is the same reason it may not name the binary or the account
    // ([ADR-0071](../../docs/decisions/ADR-0071-restrict-the-project-layer-to-the-profile-key.md)).
    if project
        && (file.child_bin.is_some()
            || file.default_account.is_some()
            || file.auto_trust_cwd.is_some())
    {
        let key = if file.child_bin.is_some() {
            "child_bin"
        } else if file.default_account.is_some() {
            "default_account"
        } else {
            "auto_trust_cwd"
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
    if let Some(value) = file.auto_trust_cwd {
        resolved.auto_trust_cwd_mut().set(value, source);
    }
    Ok(true)
}

fn apply_environment(
    resolved: &mut ResolvedConfig,
    environment: &[(OsString, OsString)],
) -> Result<(), ConfigError> {
    for (key, value) in environment {
        match key.to_str() {
            Some("CLAUDE_SESSION_RS_CHILD_BIN") => resolved
                .child_bin_mut()
                .set(PathBuf::from(value), Source::Environment),
            Some("CLAUDE_SESSION_RS_DEFAULT_ACCOUNT") => resolved.account_mut().set(
                environment_identifier("default_account", value)?,
                Source::Environment,
            ),
            Some("CLAUDE_SESSION_RS_DEFAULT_PROFILE") => resolved.profile_mut().set(
                environment_identifier("default_profile", value)?,
                Source::Environment,
            ),
            Some("CLAUDE_SESSION_RS_AUTO_TRUST_CWD") => resolved.auto_trust_cwd_mut().set(
                environment_flag("auto_trust_cwd", value)?,
                Source::Environment,
            ),
            _ => {}
        }
    }
    Ok(())
}

/// Decodes a boolean environment value, refusing anything ambiguous.
///
/// The four spellings TOML and the shell agree on, and nothing else: a value
/// the wrapper guessed at would silently answer a trust question the user meant
/// to answer themselves.
fn environment_flag(key: &'static str, value: &OsStr) -> Result<bool, ConfigError> {
    match value.to_str() {
        Some("true" | "1") => Ok(true),
        Some("false" | "0") => Ok(false),
        // Deliberately not the identifier error: there is no grammar to
        // correct, only two spellings that mean yes and two that mean no.
        _ => Err(ConfigError::Value {
            key,
            origin: format!(
                "{} (expected true, false, 1, or 0)",
                environment_spelling(key)
            ),
        }),
    }
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
        b"default_account" => "CLAUDE_SESSION_RS_DEFAULT_ACCOUNT",
        b"auto_trust_cwd" => "CLAUDE_SESSION_RS_AUTO_TRUST_CWD",
        _ => "CLAUDE_SESSION_RS_DEFAULT_PROFILE",
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::config::schema::KEYS;

    /// ADR-0013's "generation fails when a public field carries no description",
    /// enforced at test time: a document built from `KEYS` must deserialize into
    /// `FileConfig`, whose `deny_unknown_fields` rejects a described key that is
    /// not a field, and the length assertion catches a field with no key.
    #[test]
    fn every_configuration_field_has_a_described_key() {
        let mut document = String::new();
        for key in KEYS {
            use std::fmt::Write as _;
            // Typed, because a boolean field cannot decode from a quoted
            // placeholder and the point of the round trip is that it decodes.
            let value = if key.type_name == "boolean" {
                "true".to_owned()
            } else {
                "\"placeholder\"".to_owned()
            };
            let _ = writeln!(document, "{} = {value}", key.name);
        }
        let decoded: FileConfig =
            toml::from_str(&document).expect("every described key is a field");
        let set = [
            decoded.child_bin.is_some(),
            decoded.default_account.is_some(),
            decoded.default_profile.is_some(),
            decoded.auto_trust_cwd.is_some(),
        ];
        assert_eq!(
            set.iter().filter(|value| **value).count(),
            KEYS.len(),
            "a configuration field carries no KEYS entry"
        );
    }

    #[test]
    fn every_key_description_is_non_empty() {
        for key in KEYS {
            assert!(!key.description.trim().is_empty(), "{}", key.name);
        }
    }

    /// A missing file is not an error, but "which files were consulted" has to
    /// survive that, or `config` could not report it.
    #[test]
    fn an_absent_file_still_records_that_it_was_consulted() {
        let mut resolved = ResolvedConfig::defaults();
        let existed = apply_file(
            &mut resolved,
            Path::new("/nonexistent/claude-session/config.toml"),
            Source::User,
            false,
        )
        .expect("an absent file is not an error");
        assert!(!existed);
    }
}
