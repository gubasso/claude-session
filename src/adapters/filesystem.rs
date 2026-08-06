//! Narrow filesystem operations used by configuration and process resolution.

use std::{
    fs, io,
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};

/// Linux filesystem adapter.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FileSystem;

impl FileSystem {
    /// Reads a complete file.
    pub(crate) fn read(path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }
    /// Returns followed metadata.
    pub(crate) fn metadata(path: &Path) -> io::Result<fs::Metadata> {
        fs::metadata(path)
    }
    /// Resolves the candidate to an absolute physical path.
    pub(crate) fn canonicalize(path: &Path) -> io::Result<std::path::PathBuf> {
        fs::canonicalize(path)
    }
    /// Checks executable access with the kernel's permission rules.
    pub(crate) fn executable(path: &Path) -> io::Result<bool> {
        match rustix::fs::access(path, rustix::fs::Access::EXEC_OK) {
            Ok(()) => Ok(true),
            Err(error) if error == rustix::io::Errno::ACCESS => Ok(false),
            Err(error) => Err(io::Error::from_raw_os_error(error.raw_os_error())),
        }
    }
    /// Returns the stable device/inode file identity.
    pub(crate) fn identity(path: &Path) -> io::Result<(u64, u64)> {
        let data = fs::metadata(path)?;
        Ok((data.dev(), data.ino()))
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
