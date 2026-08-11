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

/// The deadline every terminal-allocating leg carries. A regression that
/// leaves the interactive child waiting must fail the case within the lane's
/// budget rather than hang the whole push hook, so the bound is explicit and
/// far above the single-digit seconds a healthy run takes.
/// `--foreground` is required, not cosmetic: without it `timeout` puts the
/// command in its own process group, which makes it a background group on the
/// allocated terminal and stops it reading from it at all.
const TERMINAL_DEADLINE: [&str; 3] = ["--foreground", "--kill-after=5", "30"];

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

    /// Runs the wrapper in its own session, so it has no controlling terminal
    /// whatever the test runner inherited. Without this the terminal predicate
    /// would be answered by the developer's terminal rather than by the
    /// wrapper, and the same command would pass headless and fail locally.
    pub(crate) fn detached_command(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new(devshell_binary("setsid"));
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
            .arg("--wait")
            .arg(devshell_binary("timeout"))
            .args(TERMINAL_DEADLINE)
            .arg(env!("CARGO_BIN_EXE_claude-session"))
            .args(arguments)
            .current_dir(self.root.path());
        command
    }
    /// Runs the wrapper under the devShell's util-linux PTY allocator.
    pub(crate) fn terminal_command(&self, arguments: &str) -> Command {
        let mut command = Command::new(devshell_binary("script"));
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
            .arg("-q")
            .arg("-e")
            .arg("-c")
            .arg(format!(
                "{} {} {} {arguments}",
                devshell_binary("timeout").display(),
                TERMINAL_DEADLINE.join(" "),
                env!("CARGO_BIN_EXE_claude-session"),
            ))
            .arg("/dev/null")
            .current_dir(self.root.path());
        command
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
    /// Creates a usable child-owned saved-login fixture without reading it.
    pub(crate) fn initialize_login(&self, name: &str) {
        let account = self.state().join("accounts").join(name);
        let config = account.join("config");
        fs::create_dir_all(&config).expect("account fixture");
        fs::set_permissions(&account, fs::Permissions::from_mode(0o700)).expect("account mode");
        fs::set_permissions(&config, fs::Permissions::from_mode(0o700)).expect("config mode");
        fs::write(config.join(".credentials.json"), b"child-owned-fixture")
            .expect("credential fixture");
        fs::write(
            account.join("auth-mode.json"),
            b"{\"mode\":\"login\",\"recorded_at\":\"2026-08-11T00:00:00Z\"}\n",
        )
        .expect("mode fixture");
        fs::set_permissions(
            account.join("auth-mode.json"),
            fs::Permissions::from_mode(0o600),
        )
        .expect("metadata mode");
    }
}

/// Resolves a util-linux helper the pinned devShell provides. The panic names
/// the flake, because a missing binary means the shell was bypassed rather
/// than that the test is wrong.
pub(crate) fn devshell_binary(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| {
            panic!("{name} is missing from PATH; run inside the pinned devShell (flake.nix)")
        })
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

/// The child argv of every invocation, in order. The `argv` record is
/// truncated per invocation, so only this history can distinguish a run that
/// probed the version and then launched from one that only launched. An
/// invocation carrying no arguments is indistinguishable from the record
/// separator and is not represented.
pub(crate) fn read_invocations(path: &Path) -> Vec<Vec<Vec<u8>>> {
    let Ok(bytes) = fs::read(path) else {
        return Vec::new();
    };
    let mut invocations = Vec::new();
    let mut current = Vec::new();
    for field in bytes.split(|byte| *byte == 0) {
        if field.is_empty() {
            if !current.is_empty() {
                invocations.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(field.to_vec());
    }
    if !current.is_empty() {
        invocations.push(current);
    }
    invocations
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
