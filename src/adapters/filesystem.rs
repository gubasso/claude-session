//! Narrow filesystem operations used by configuration and process resolution.

use std::{
    fs, io,
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
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
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir_all(path)?,
            Err(error) => return Err(error),
        }
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
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
