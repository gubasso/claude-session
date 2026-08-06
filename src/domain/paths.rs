//! Absolute XDG-owned application paths.

use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use crate::error::DomainError;

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
    /// Returns the data namespace.
    pub(crate) fn data(&self) -> &Path {
        &self.data
    }
    /// Returns the cache namespace.
    pub(crate) fn cache(&self) -> &Path {
        &self.cache
    }
}
