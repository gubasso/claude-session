use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
};

fn write_nul(path: PathBuf, values: impl IntoIterator<Item = OsString>) {
    let mut file = fs::File::create(path).expect("record file");
    for value in values {
        file.write_all(value.as_os_str().as_bytes()).expect("record value");
        file.write_all(&[0]).expect("separator");
    }
}

fn main() {
    let record = PathBuf::from(std::env::var_os("CS_TEST_RECORD_DIR").expect("record dir"));
    fs::create_dir_all(&record).expect("record directory");
    write_nul(record.join("argv"), std::env::args_os());
    write_nul(
        record.join("environ"),
        std::env::vars_os().flat_map(|(key, value)| [key, value]),
    );
    write_nul(
        record.join("cwd"),
        [std::env::current_dir().expect("cwd").into_os_string()],
    );
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    {
        let mut history = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(record.join("invocations"))
            .expect("history");
        for argument in &arguments {
            history
                .write_all(argument.as_os_str().as_bytes())
                .expect("history argument");
            history.write_all(&[0]).expect("history separator");
        }
        history
            .write_all(&[0])
            .expect("history invocation separator");
    }
    if let Some(marker) = std::env::var_os("CS_TEST_MARKER_PATH") {
        let visible = if PathBuf::from(marker).is_file() {
            b"1\n".as_slice()
        } else {
            b"0\n".as_slice()
        };
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(record.join("marker-visible"))
            .expect("marker record")
            .write_all(visible)
            .expect("marker visibility");
    }
    if arguments.as_slice() == [OsString::from("auth"), OsString::from("login")]
        && std::env::var_os("CS_TEST_CREATE_CREDENTIAL").is_some()
    {
        let config = PathBuf::from(std::env::var_os("CLAUDE_CONFIG_DIR").expect("config dir"));
        fs::create_dir_all(&config).expect("config directory");
        fs::write(
            config.join(".credentials.json"),
            b"child-owned-login-bytes",
        )
        .expect("credential");
    }
    // Simulates a second run committing this account while the interactive
    // login of the run under test is still going. The cleanup on failure must
    // notice and leave the committed account alone.
    if std::env::var_os("CS_TEST_COMMIT_METADATA").is_some() {
        let config = PathBuf::from(std::env::var_os("CLAUDE_CONFIG_DIR").expect("config dir"));
        let account = config.parent().expect("account directory").to_path_buf();
        fs::create_dir_all(&account).expect("account directory");
        fs::write(
            account.join("auth-mode.json"),
            b"{\"mode\":\"login\",\"recorded_at\":\"2026-08-11T00:00:00Z\"}\n",
        )
        .expect("committed metadata");
    }
    // Simulates drift arriving while the interactive login runs: the account's
    // configuration directory is replaced by a link to somewhere else. The
    // wrapper validated the directory before the child started, so only a
    // revalidation after it returns can catch this.
    if let Some(target) = std::env::var_os("CS_TEST_SWAP_CONFIG") {
        let config = PathBuf::from(std::env::var_os("CLAUDE_CONFIG_DIR").expect("config dir"));
        let _ = fs::remove_dir_all(&config);
        std::os::unix::fs::symlink(&target, &config).expect("swap config");
    }
    // Records whether the wrapper handed this probe a token, and which one, so
    // a test can prove the candidate reached the child through its environment
    // rather than through argv. The argv record above already proves the
    // negative half.
    if arguments.first() == Some(&OsString::from("auth"))
        && arguments.get(1) == Some(&OsString::from("status"))
    {
        let seen = std::env::var_os("CLAUDE_CODE_OAUTH_TOKEN").unwrap_or_default();
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(record.join("probe-token"))
            .expect("probe record")
            .write_all(seen.as_os_str().as_bytes())
            .expect("probe token");
    }
    let prefix = if arguments.as_slice() == [OsString::from("--version")] {
        Some("VERSION")
    } else if arguments.as_slice() == [OsString::from("doctor")] {
        Some("DOCTOR")
    } else if arguments.first() == Some(&OsString::from("auth"))
        && arguments.get(1) == Some(&OsString::from("status"))
    {
        Some("PROBE")
    } else if arguments.as_slice() == [OsString::from("setup-token")] {
        Some("SETUP_TOKEN")
    } else {
        None
    };
    let selected = |suffix: &str| {
        prefix
            .and_then(|prefix| std::env::var_os(format!("CS_TEST_{prefix}_{suffix}")))
            .or_else(|| std::env::var_os(format!("CS_TEST_{suffix}")))
    };
    if let Some(bytes) = selected("STDOUT") {
        io::stdout().write_all(bytes.as_os_str().as_bytes()).expect("stdout");
    } else if prefix == Some("VERSION") {
        io::stdout().write_all(b"2.1.211 (Claude Code)\n").expect("stdout");
    }
    if let Some(bytes) = selected("STDERR") {
        io::stderr().write_all(bytes.as_os_str().as_bytes()).expect("stderr");
    }
    if std::env::var_os("CS_TEST_ABORT").is_some() { std::process::abort(); }
    let code = selected("EXIT")
        .and_then(|value| value.to_str().and_then(|value| value.parse().ok()))
        .unwrap_or(0);
    std::process::exit(code);
}
