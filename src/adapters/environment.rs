//! The only adapter allowed to read process-global environment state.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

/// Read-only process environment port.
pub(crate) trait Environment {
    /// Returns captured arguments including `argv[0]`.
    fn args(&self) -> &[OsString];
    /// Returns the captured environment.
    fn variables(&self) -> &[(OsString, OsString)];
    /// Returns the captured working directory.
    fn current_dir(&self) -> &Path;
    /// Returns the captured current executable path.
    fn current_exe(&self) -> &Path;
}

/// One immutable snapshot of ambient process state.
#[derive(Clone, Debug)]
pub(crate) struct SystemEnvironment {
    args: Vec<OsString>,
    variables: Vec<(OsString, OsString)>,
    current_dir: PathBuf,
    current_exe: PathBuf,
}

impl SystemEnvironment {
    /// Captures ambient state once at the adapter boundary.
    pub(crate) fn capture() -> Self {
        Self {
            args: std::env::args_os().collect(),
            variables: std::env::vars_os().collect(),
            current_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            current_exe: std::env::current_exe()
                .unwrap_or_else(|_| PathBuf::from("/proc/self/exe")),
        }
    }
}

impl Environment for SystemEnvironment {
    fn args(&self) -> &[OsString] {
        &self.args
    }
    fn variables(&self) -> &[(OsString, OsString)] {
        &self.variables
    }
    fn current_dir(&self) -> &Path {
        &self.current_dir
    }
    fn current_exe(&self) -> &Path {
        &self.current_exe
    }
}
