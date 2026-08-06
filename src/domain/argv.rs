//! Pure, total partitioning of wrapper and child OS-string arguments.

use std::ffi::OsString;

use crate::error::DomainError;

/// The raw wrapper prefix and untouched child suffix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Partition {
    wrapper: Vec<OsString>,
    child: Vec<OsString>,
    sentinel: bool,
}

impl Partition {
    /// Returns the wrapper-owned prefix, including `argv[0]`.
    #[cfg(test)]
    pub(crate) fn wrapper(&self) -> &[OsString] {
        &self.wrapper
    }
    /// Returns the untouched child suffix.
    #[cfg(test)]
    pub(crate) fn child(&self) -> &[OsString] {
        &self.child
    }
    /// Reports whether a `--` sentinel closed the wrapper prefix.
    ///
    /// The sentinel is consumed rather than forwarded, so nothing downstream
    /// can recover it from the suffix alone. A caller that treats a leading
    /// child token as a wrapper verb has to ask.
    pub(crate) const fn sentinel(&self) -> bool {
        self.sentinel
    }
    /// Moves out both argument zones.
    pub(crate) fn into_parts(self) -> (Vec<OsString>, Vec<OsString>) {
        (self.wrapper, self.child)
    }
}

/// Splits leading claimed flags from opaque child arguments.
pub(crate) fn split(arguments: &[OsString]) -> Result<Partition, DomainError> {
    if arguments.is_empty() {
        return Ok(Partition {
            wrapper: Vec::new(),
            child: Vec::new(),
            sentinel: false,
        });
    }
    let mut wrapper = vec![arguments[0].clone()];
    let mut index = 1;
    let mut sentinel = false;
    let mut seen_single = std::collections::BTreeSet::new();
    let mut verbose = false;
    let mut quiet = false;
    while index < arguments.len() {
        let bytes = crate::util::os::bytes(arguments[index].as_os_str());
        if bytes == b"--" {
            index += 1;
            sentinel = true;
            break;
        }
        let (name, takes_value, attached) = claimed(bytes);
        let Some(name) = name else {
            break;
        };
        if name == "verbose" {
            verbose = true;
        }
        if name == "quiet" {
            quiet = true;
        }
        if verbose && quiet {
            return Err(DomainError::InvalidArguments(
                "--quiet conflicts with --verbose".into(),
            ));
        }
        if !matches!(name, "verbose" | "quiet") && !seen_single.insert(name) {
            return Err(DomainError::InvalidArguments(format!(
                "--{name} may not be repeated"
            )));
        }
        wrapper.push(arguments[index].clone());
        if takes_value && !attached {
            let Some(value) = arguments.get(index + 1) else {
                return Err(DomainError::InvalidArguments(format!(
                    "--{name} requires a value"
                )));
            };
            if crate::util::os::bytes(value.as_os_str()).starts_with(b"-") {
                return Err(DomainError::InvalidArguments(format!(
                    "--{name} requires a value"
                )));
            }
            wrapper.push(value.clone());
            index += 1;
        }
        index += 1;
    }
    Ok(Partition {
        wrapper,
        child: arguments[index..].to_vec(),
        sentinel,
    })
}

fn claimed(bytes: &[u8]) -> (Option<&'static str>, bool, bool) {
    match bytes {
        b"--verbose" => (Some("verbose"), false, false),
        b"--quiet" | b"-q" => (Some("quiet"), false, false),
        b"--version" | b"-V" => (Some("version"), false, false),
        b"--help" | b"-h" => (Some("help"), false, false),
        b"--config" => (Some("config"), true, false),
        b"--account" => (Some("account"), true, false),
        b"--profile" => (Some("profile"), true, false),
        _ if bytes.starts_with(b"--config=") => (Some("config"), true, true),
        _ if bytes.starts_with(b"--account=") => (Some("account"), true, true),
        _ if bytes.starts_with(b"--profile=") => (Some("profile"), true, true),
        _ => (None, false, false),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;

    #[test]
    fn golden_argv_table() {
        let raw = vec![
            OsString::from("wrapper"),
            OsString::from("--"),
            OsString::new(),
            OsString::from_vec(vec![0x66, 0x80, 0x6f]),
            OsString::from("--"),
        ];
        let result = split(&raw).expect("valid split");
        assert_eq!(result.wrapper(), &[OsString::from("wrapper")]);
        assert_eq!(result.child(), &raw[2..]);
        let empty = split(&[]).expect("total");
        assert!(empty.wrapper().is_empty());
    }

    #[test]
    fn spelling_matrix() {
        for flag in [
            "--verbose",
            "--quiet",
            "-q",
            "--version",
            "-V",
            "--help",
            "-h",
        ] {
            let raw = vec!["w".into(), flag.into(), "child".into()];
            assert_eq!(split(&raw).expect("split").wrapper().len(), 2);
        }
        for raw_flag in ["--Verbose", "--verb", "-qv", "--configg=x"] {
            let raw = vec!["w".into(), raw_flag.into()];
            assert_eq!(
                split(&raw).expect("split").child(),
                &[OsString::from(raw_flag)]
            );
        }
        assert!(split(&["w".into(), "--config".into(), "-x".into()]).is_err());
        assert_eq!(
            split(&["w".into(), "--config=x".into()])
                .expect("attached")
                .wrapper()
                .len(),
            2
        );
    }

    #[test]
    fn split_is_total() {
        for bytes in [vec![], vec![0], vec![0x80], b"anything".to_vec()] {
            let _ = split(&[OsString::from("w"), OsString::from_vec(bytes)]);
        }
    }
}
