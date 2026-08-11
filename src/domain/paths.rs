//! Absolute XDG-owned application paths.

use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use crate::{domain::identifier::Identifier, error::DomainError};

/// Absolute XDG namespace paths owned by the wrapper.
#[derive(Clone, Debug)]
pub(crate) struct XdgPaths {
    config: PathBuf,
    state: PathBuf,
    data: PathBuf,
    cache: PathBuf,
}

impl XdgPaths {
    /// Resolves all four bases from an OS-string environment snapshot.
    pub(crate) fn resolve(environment: &[(OsString, OsString)]) -> Result<Self, DomainError> {
        let map: HashMap<&OsStr, &OsStr> = environment
            .iter()
            .map(|(key, value)| (key.as_os_str(), value.as_os_str()))
            .collect();
        let home = map
            .get(OsStr::new("HOME"))
            .map(|value| PathBuf::from(*value));
        let base = |name: &str, fallback: &str| -> Result<PathBuf, DomainError> {
            if let Some(value) = map.get(OsStr::new(name)) {
                let path = PathBuf::from(value);
                if !value.is_empty() && path.is_absolute() {
                    return Ok(path);
                }
            }
            let Some(home) = &home else {
                return Err(DomainError::MissingHome);
            };
            Ok(home.join(fallback))
        };
        Ok(Self {
            config: base("XDG_CONFIG_HOME", ".config")?.join("claude-session"),
            state: base("XDG_STATE_HOME", ".local/state")?.join("claude-session"),
            data: base("XDG_DATA_HOME", ".local/share")?.join("claude-session"),
            cache: base("XDG_CACHE_HOME", ".cache")?.join("claude-session"),
        })
    }

    /// Returns the configuration namespace.
    pub(crate) fn config(&self) -> &Path {
        &self.config
    }
    /// Returns the state namespace.
    pub(crate) fn state(&self) -> &Path {
        &self.state
    }
    // Resolving all four namespaces is the XDG contract itself, so these two are
    // carried without a reader rather than dropped. The allow is scoped to the
    // item so a genuinely dead addition elsewhere still reports.
    /// Returns the data namespace.
    #[allow(dead_code, reason = "no namespace has a durable artifact yet")]
    pub(crate) fn data(&self) -> &Path {
        &self.data
    }
    /// Returns the cache namespace.
    #[allow(dead_code, reason = "no namespace has a discardable artifact yet")]
    pub(crate) fn cache(&self) -> &Path {
        &self.cache
    }

    /// Returns the accounts collection directory.
    pub(crate) fn accounts(&self) -> PathBuf {
        self.state.join("accounts")
    }
    /// Returns one account's directory.
    pub(crate) fn account(&self, account: &Identifier) -> PathBuf {
        self.accounts().join(account.as_str())
    }
    /// Returns the child-owned native configuration directory for one account.
    ///
    /// The wrapper creates it; the child writes everything inside it.
    pub(crate) fn account_config(&self, account: &Identifier) -> PathBuf {
        self.account(account).join("config")
    }
    /// Returns one account's authentication-mode metadata.
    pub(crate) fn account_auth_mode(&self, account: &Identifier) -> PathBuf {
        self.account(account).join("auth-mode.json")
    }
    /// Returns one account's future wrapper-owned token path.
    pub(crate) fn account_oauth_token(&self, account: &Identifier) -> PathBuf {
        self.account(account).join("oauth-token")
    }
    /// Returns the child-owned saved-login path without opening it.
    pub(crate) fn account_credentials(&self, account: &Identifier) -> PathBuf {
        self.account_config(account).join(".credentials.json")
    }
    /// Returns the last-used account marker.
    pub(crate) fn last_account(&self) -> PathBuf {
        self.state.join("state").join("last-account")
    }
    /// Returns the composed-settings store directory.
    pub(crate) fn composed(&self) -> PathBuf {
        self.state.join("composed")
    }
    /// Returns the profile document for one profile name.
    pub(crate) fn profile_file(&self, profile: &Identifier) -> PathBuf {
        self.config
            .join("profiles")
            .join(format!("{}.yaml", profile.as_str()))
    }
    /// Returns the settings piece file for one piece name.
    pub(crate) fn piece_file(&self, piece: &Identifier) -> PathBuf {
        self.config
            .join("settings")
            .join(format!("{}.json", piece.as_str()))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn fixture() -> XdgPaths {
        XdgPaths::resolve(&[
            (OsString::from("XDG_CONFIG_HOME"), OsString::from("/c")),
            (OsString::from("XDG_STATE_HOME"), OsString::from("/s")),
            (OsString::from("XDG_DATA_HOME"), OsString::from("/d")),
            (OsString::from("XDG_CACHE_HOME"), OsString::from("/k")),
        ])
        .expect("absolute bases resolve")
    }

    /// A compiler-checked copy of the artifact table's path column, so a
    /// renamed directory fails here rather than silently relocating a user's
    /// durable state.
    #[test]
    fn every_managed_path_matches_the_artifact_table() {
        let paths = fixture();
        let work = "work".parse().expect("identifier");
        assert_eq!(paths.accounts(), Path::new("/s/claude-session/accounts"));
        assert_eq!(
            paths.account(&work),
            Path::new("/s/claude-session/accounts/work")
        );
        assert_eq!(
            paths.account_config(&work),
            Path::new("/s/claude-session/accounts/work/config")
        );
        assert_eq!(
            paths.account_auth_mode(&work),
            Path::new("/s/claude-session/accounts/work/auth-mode.json")
        );
        assert_eq!(
            paths.account_oauth_token(&work),
            Path::new("/s/claude-session/accounts/work/oauth-token")
        );
        assert_eq!(
            paths.account_credentials(&work),
            Path::new("/s/claude-session/accounts/work/config/.credentials.json")
        );
        assert_eq!(
            paths.last_account(),
            Path::new("/s/claude-session/state/last-account")
        );
        assert_eq!(paths.composed(), Path::new("/s/claude-session/composed"));
        assert_eq!(
            paths.profile_file(&work),
            Path::new("/c/claude-session/profiles/work.yaml")
        );
        assert_eq!(
            paths.piece_file(&work),
            Path::new("/c/claude-session/settings/work.json")
        );
    }
}
