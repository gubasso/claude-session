use std::{ffi::OsString, fs, io::{self, Write}, os::unix::ffi::OsStrExt, path::PathBuf};

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
    write_nul(record.join("environ"), std::env::vars_os().flat_map(|(key, value)| [key, value]));
    write_nul(record.join("cwd"), [std::env::current_dir().expect("cwd").into_os_string()]);
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    let prefix = if arguments.as_slice() == [OsString::from("--version")] {
        Some("VERSION")
    } else if arguments.as_slice() == [OsString::from("doctor")] {
        Some("DOCTOR")
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
