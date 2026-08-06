//! Lossless Unix OS-string byte access.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

/// Returns the exact bytes of a Unix OS string.
pub(crate) fn bytes(value: &OsStr) -> &[u8] {
    value.as_bytes()
}
