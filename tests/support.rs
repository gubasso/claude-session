#![allow(
    dead_code,
    clippy::expect_used,
    clippy::pedantic,
    clippy::nursery,
    clippy::redundant_pub_crate
)]

use std::{
    ffi::{OsStr, OsString},
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) struct Harness {
    root: tempfile::TempDir,
    child: PathBuf,
}

impl Harness {
    pub(crate) fn new() -> Self {
        let root = tempfile::tempdir().expect("temporary test tree");
        let child = root.path().join("recording-child");
        let status = Command::new("rustc")
            .arg("--edition=2024")
            .arg("tests/support/recording_child.rs")
            .arg("-o")
            .arg(&child)
            .status()
            .expect("run pinned rustc");
        assert!(status.success(), "recording child must compile");
        Self { root, child }
    }

    pub(crate) fn root(&self) -> &Path {
        self.root.path()
    }
    pub(crate) fn child(&self) -> &Path {
        &self.child
    }
    pub(crate) fn record_dir(&self) -> PathBuf {
        self.root.path().join("record")
    }
    pub(crate) fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_claude-session"));
        command.env_clear();
        for (name, suffix) in [
            ("HOME", "home"),
            ("XDG_CONFIG_HOME", "config"),
            ("XDG_STATE_HOME", "state"),
            ("XDG_DATA_HOME", "data"),
            ("XDG_CACHE_HOME", "cache"),
        ] {
            let path = self.root.path().join(suffix);
            fs::create_dir_all(&path).expect("XDG fixture");
            command.env(name, path);
        }
        fs::create_dir_all(self.record_dir()).expect("record directory");
        command
            .env("CLAUDE_SESSION_CHILD_BIN", &self.child)
            .env("CS_TEST_RECORD_DIR", self.record_dir())
            .current_dir(self.root.path());
        command
    }

    pub(crate) fn assert_command(&self) -> assert_cmd::Command {
        assert_cmd::Command::from_std(self.command())
    }

    /// The wrapper-managed state namespace inside this fixture's XDG state base.
    pub(crate) fn state(&self) -> PathBuf {
        self.root.path().join("state/claude-session")
    }
    /// The wrapper's config namespace inside this fixture's XDG config base.
    pub(crate) fn config_base(&self) -> PathBuf {
        self.root.path().join("config/claude-session")
    }
    /// Writes a settings piece the user would have authored.
    pub(crate) fn write_piece(&self, name: &str, json: &str) {
        let directory = self.config_base().join("settings");
        fs::create_dir_all(&directory).expect("settings fixture");
        fs::write(directory.join(format!("{name}.json")), json).expect("piece fixture");
    }
    /// Writes a profile document the user would have authored.
    pub(crate) fn write_profile(&self, name: &str, yaml: &str) {
        let directory = self.config_base().join("profiles");
        fs::create_dir_all(&directory).expect("profiles fixture");
        fs::write(directory.join(format!("{name}.yaml")), yaml).expect("profile fixture");
    }
}

/// The permission bits of a path, read without following a symbolic link.
pub(crate) fn mode(path: &Path) -> u32 {
    fs::symlink_metadata(path)
        .expect("fixture metadata")
        .permissions()
        .mode()
        & 0o7777
}

/// The names in one directory, sorted so an assertion is order-independent.
pub(crate) fn entries(path: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(path)
        .expect("fixture directory")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

pub(crate) fn read_nul(path: &Path) -> Vec<Vec<u8>> {
    let bytes = fs::read(path).expect("recording file");
    let mut values: Vec<_> = bytes.split(|byte| *byte == 0).map(<[u8]>::to_vec).collect();
    if values.last().is_some_and(Vec::is_empty) {
        values.pop();
    }
    values
}

pub(crate) fn make_executable(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).expect("fixture file");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("fixture mode");
}

pub(crate) fn os(bytes: Vec<u8>) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes)
}
pub(crate) fn bytes(value: &OsStr) -> &[u8] {
    use std::os::unix::ffi::OsStrExt;
    value.as_bytes()
}
