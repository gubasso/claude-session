//! Narrow filesystem operations used by configuration and process resolution.

use std::{
    fs, io,
    os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};

/// What the resolution ladder needs to know about an existing path.
///
/// Narrower than `std::fs::Metadata` on purpose: a port that hands back a type
/// only `std::fs` can construct cannot be faked, which defeats the point of
/// having a port at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FileFacts {
    /// Whether the path is a regular file.
    pub(crate) regular: bool,
    /// The permission bits.
    pub(crate) mode: u32,
}

/// What the storage guard needs about an existing path, read without following.
///
/// Separate from [`FileFacts`] because the two answer different questions: that
/// one describes a followed candidate the resolution ladder may run, this one
/// describes the link itself, which is the only reading in which "is it a
/// symbolic link" is answerable at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PathFacts {
    /// Whether the path itself is a symbolic link.
    pub(crate) symlink: bool,
    /// Whether the path itself is a directory.
    pub(crate) directory: bool,
    /// Whether the path itself is a regular file.
    pub(crate) regular: bool,
    /// The owning user id.
    pub(crate) uid: u32,
    /// The permission bits, masked to `0o7777`.
    pub(crate) mode: u32,
}

/// The filesystem port.
///
/// A service depends on this rather than on the system implementation, so a
/// test can substitute a fake and exercise a resolution ladder without laying
/// down a tree or forking a process.
pub(crate) trait FileSystem {
    /// Returns the facts about a followed path.
    fn describe(&self, path: &Path) -> io::Result<FileFacts>;
    /// Resolves the candidate to an absolute physical path.
    fn canonicalize(&self, path: &Path) -> io::Result<std::path::PathBuf>;
    /// Checks executable access with the kernel's permission rules.
    fn executable(&self, path: &Path) -> io::Result<bool>;
    /// Returns the stable device/inode file identity.
    fn identity(&self, path: &Path) -> io::Result<(u64, u64)>;
}

/// Linux filesystem adapter.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SystemFileSystem;

impl FileSystem for SystemFileSystem {
    fn describe(&self, path: &Path) -> io::Result<FileFacts> {
        let data = fs::metadata(path)?;
        Ok(FileFacts {
            regular: data.is_file(),
            mode: data.permissions().mode(),
        })
    }
    fn canonicalize(&self, path: &Path) -> io::Result<std::path::PathBuf> {
        fs::canonicalize(path)
    }
    fn executable(&self, path: &Path) -> io::Result<bool> {
        match rustix::fs::access(path, rustix::fs::Access::EXEC_OK) {
            Ok(()) => Ok(true),
            Err(error) if error == rustix::io::Errno::ACCESS => Ok(false),
            Err(error) => Err(io::Error::from_raw_os_error(error.raw_os_error())),
        }
    }
    fn identity(&self, path: &Path) -> io::Result<(u64, u64)> {
        let data = fs::metadata(path)?;
        Ok((data.dev(), data.ino()))
    }
}

// Reading a configuration file and preparing the log namespace happen once each,
// from a caller that owns the real filesystem by definition, so they stay
// associated functions rather than widening the port with methods no fake needs.
impl SystemFileSystem {
    /// Reads a complete file.
    pub(crate) fn read(path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }
    /// Creates the state namespace with private permissions.
    ///
    /// Logging calls this before a context or the guard's error type exists, so
    /// it stays best-effort and narrow: the unmanaged ancestors are created
    /// however the umask says, and only the `claude-session-rs` component itself
    /// is refused, validated, and made private.
    pub(crate) fn create_private_dir(path: &Path) -> io::Result<()> {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink()
                    || !metadata.is_dir()
                    || metadata.uid() != rustix::process::getuid().as_raw()
                {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "state namespace is not a private owned directory",
                    ));
                }
                return fs::set_permissions(path, fs::Permissions::from_mode(0o700));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        // Ancestors above the namespace directory are the operating system's or
        // the user's, and are outside the wrapper's policy.
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Self::create_dir_private(path)
    }

    /// Reads a path's own facts without following a symbolic link.
    ///
    /// `Ok(None)` is the absent case, which every caller has to distinguish
    /// from a refusal: a component that does not exist yet is not a failure,
    /// it is the thing about to be created.
    pub(crate) fn look(path: &Path) -> io::Result<Option<PathFacts>> {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                let file_type = metadata.file_type();
                Ok(Some(PathFacts {
                    symlink: file_type.is_symlink(),
                    directory: file_type.is_dir(),
                    regular: file_type.is_file(),
                    uid: metadata.uid(),
                    mode: metadata.permissions().mode() & 0o7777,
                }))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Creates one directory component private, rather than creating then fixing.
    ///
    /// One component, never `create_dir_all`: the guard validates each ancestor
    /// in turn before creating the next, and the recursive form would create
    /// intermediate components unvalidated and at the umask's mode.
    pub(crate) fn create_dir_private(path: &Path) -> io::Result<()> {
        match fs::DirBuilder::new().mode(0o700).create(path) {
            // `mkdir(2)` applies the process umask to the requested mode, so an
            // owner-masking umask would leave the component narrower than
            // `0700` and unusable — the next walk step could not even stat
            // through it. Settling the mode is not the "create then correct"
            // the owner page rules out: that rule forbids a permissive window,
            // and this only ever widens back toward the documented mode on a
            // directory this call just created and therefore owns.
            Ok(()) => fs::set_permissions(path, fs::Permissions::from_mode(0o700)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// Corrects a directory's mode after its own non-following check.
    ///
    /// Path-based, because a directory correction has no held descriptor. The
    /// immediately preceding non-following check is what makes it safe enough
    /// for the accident model [ADR-0061] scopes; a process running as this user
    /// is explicitly not the threat.
    ///
    /// [ADR-0061]: ../../docs/decisions/ADR-0061-protect-storage-from-accidental-local-drift.md
    /// Creates one symbolic link, treating an existing one as done.
    ///
    /// Only the guard calls this, and only at a name the wrapper declares, so
    /// the link a run finds is the link a run would have made
    /// ([ADR-0103](../../docs/decisions/ADR-0103-permit-a-declared-link.md)).
    /// `AlreadyExists` is success rather than a race to resolve: the caller has
    /// just validated that whatever sits there is the declared link pointing at
    /// the declared target, and a second run computing the same pair is the
    /// normal case rather than a conflict.
    ///
    /// No mode is set. A symbolic link carries the kernel's own permissions,
    /// `chmod(2)` follows it to the target, and the symlink-safe form is out of
    /// reach for the same reason `xdg-storage.md#how-a-path-is-validated`
    /// records against `fchmodat(2)`.
    pub(crate) fn create_symlink(target: &Path, path: &Path) -> io::Result<()> {
        match std::os::unix::fs::symlink(target, path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
            Err(error) => Err(error),
        }
    }
    /// Reads one symbolic link's target without resolving it.
    ///
    /// Verbatim, never canonicalized: the guard compares it against the target
    /// the wrapper recorded, and a resolved form would compare something the
    /// wrapper never wrote.
    pub(crate) fn read_link(path: &Path) -> io::Result<std::path::PathBuf> {
        fs::read_link(path)
    }
    pub(crate) fn set_dir_mode(path: &Path) -> io::Result<()> {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
    }

    /// Corrects a mode through a held descriptor.
    ///
    /// Required wherever the wrapper holds one: a path-based `chmod(2)`
    /// dereferences a symbolic link, and the symlink-safe form —
    /// `AT_SYMLINK_NOFOLLOW` on `fchmodat(2)` — needs glibc 2.32 and Linux 6.5.
    pub(crate) fn set_mode_at(handle: &fs::File, mode: u32) -> io::Result<()> {
        handle.set_permissions(fs::Permissions::from_mode(mode))
    }

    /// Opens a wrapper-owned private file, validating from the open handle.
    ///
    /// The file is opened once, validated again from that handle, and read from
    /// the same handle, so nothing swapped between the check and the read is
    /// what gets read.
    pub(crate) fn open_private_file(path: &Path) -> io::Result<fs::File> {
        let handle = fs::OpenOptions::new().read(true).open(path)?;
        let metadata = handle.metadata()?;
        if !metadata.file_type().is_file() || metadata.uid() != rustix::process::getuid().as_raw() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "not a private owned regular file",
            ));
        }
        if metadata.permissions().mode() & 0o7777 != 0o600 {
            Self::set_mode_at(&handle, 0o600)?;
        }
        Ok(handle)
    }

    /// Returns the file names in one directory.
    pub(crate) fn dir_entries(path: &Path) -> io::Result<Vec<std::ffi::OsString>> {
        fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect()
    }

    /// Reports whether a process id currently exists.
    ///
    /// A pid owned by another user counts as live: it exists, which is the only
    /// question the orphan sweep asks.
    pub(crate) fn process_is_live(pid: u32) -> bool {
        let Ok(raw) = i32::try_from(pid) else {
            return false;
        };
        rustix::process::Pid::from_raw(raw).is_some_and(|pid| {
            !matches!(
                rustix::process::test_kill_process(pid),
                Err(rustix::io::Errno::SRCH)
            )
        })
    }
    /// Opens a private append-only log file.
    pub(crate) fn open_private_log(path: &Path) -> io::Result<fs::File> {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink()
                    || !metadata.file_type().is_file()
                    || metadata.file_type().is_socket()
                    || metadata.uid() != rustix::process::getuid().as_raw()
                {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "log is not a private owned regular file",
                    ));
                }
                fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(path)
    }
}
