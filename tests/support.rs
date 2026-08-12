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
        let mut command = Command::new(env!("CARGO_BIN_EXE_claude-session-rs"));
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

    /// Writes a harmless one-piece profile for tests whose subject is another
    /// launch axis.
    pub(crate) fn initialize_companion_profile(&self) {
        self.write_piece("companion", "{}\n");
        self.write_profile("companion", "layers:\n  - companion\n");
    }

    /// Writes a usable token-mode account that never needs a version probe.
    pub(crate) fn initialize_companion_account(&self) {
        self.initialize_token("companion", b"companion-token", b"companion-token");
    }

    /// Returns a passthrough command with both test-only session axes selected.
    pub(crate) fn bound_command(&self) -> Command {
        self.initialize_companion_account();
        self.initialize_companion_profile();
        let mut command = self.command();
        command.args(["--account", "companion", "--profile", "companion"]);
        command
    }

    /// Returns a command with only the test-only account axis selected.
    pub(crate) fn companion_account_command(&self) -> Command {
        self.initialize_companion_account();
        let mut command = self.command();
        command.args(["--account", "companion"]);
        command
    }

    /// Returns a command with only the test-only profile axis selected.
    pub(crate) fn companion_profile_command(&self) -> Command {
        self.initialize_companion_profile();
        let mut command = self.command();
        command.args(["--profile", "companion"]);
        command
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
            .arg(env!("CARGO_BIN_EXE_claude-session-rs"))
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
                env!("CARGO_BIN_EXE_claude-session-rs"),
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

    /// Lays down a token account, optionally with metadata describing a
    /// different token than the one on disk.
    ///
    /// The mismatching form is the crash-between-renames window the rotation
    /// order deliberately makes survivable, and it is the only way to reach the
    /// torn-pair branch without racing a real rotation.
    pub(crate) fn initialize_token(&self, name: &str, token: &[u8], described: &[u8]) {
        self.initialize_token_in(&self.state(), name, token, described);
    }

    /// Lays down a token account under an explicit state namespace.
    pub(crate) fn initialize_token_in(
        &self,
        state: &Path,
        name: &str,
        token: &[u8],
        described: &[u8],
    ) {
        let account = state.join("accounts").join(name);
        let config = account.join("config");
        fs::create_dir_all(&config).expect("account fixture");
        for path in [&account, &config] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).expect("directory mode");
        }
        fs::write(account.join("oauth-token"), token).expect("token fixture");
        fs::set_permissions(
            account.join("oauth-token"),
            fs::Permissions::from_mode(0o600),
        )
        .expect("token mode");
        fs::write(
            account.join("auth-mode.json"),
            format!(
                concat!(
                    "{{\"mode\":\"token\",\"recorded_at\":\"2026-08-11T00:00:00Z\",",
                    "\"fingerprint\":\"{}\"}}\n"
                ),
                fingerprint(described)
            ),
        )
        .expect("mode fixture");
        fs::set_permissions(
            account.join("auth-mode.json"),
            fs::Permissions::from_mode(0o600),
        )
        .expect("metadata mode");
    }
}

/// The eight leading hexadecimal characters of a token's SHA-256.
///
/// Reimplemented from the specification rather than imported: the binary has no
/// library target, and a fixture that called the same code it verifies would
/// agree with a wrong implementation.
pub(crate) fn fingerprint(token: &[u8]) -> String {
    let digest = sha256(token);
    digest
        .iter()
        .take(4)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A minimal SHA-256, sufficient for fixture-sized inputs.
fn sha256(message: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let mut hash: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let mut padded = message.to_vec();
    let bits = (message.len() as u64) * 8;
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bits.to_be_bytes());
    for block in padded.chunks_exact(64) {
        let mut w = [0_u32; 64];
        for (index, chunk) in block.chunks_exact(4).enumerate() {
            w[index] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = hash;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (slot, value) in hash.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
    let mut digest = [0_u8; 32];
    for (chunk, value) in digest.chunks_exact_mut(4).zip(hash) {
        chunk.copy_from_slice(&value.to_be_bytes());
    }
    digest
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
