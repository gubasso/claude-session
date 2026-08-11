//! Byte-exactness helpers for paths that reach a JSON document.
//!
//! One owner, because every report facing the same problem has to answer it the
//! same way: the provenance sidecar and the wrapper's own reports all carry
//! paths, and two encoders would eventually disagree about one path.

use std::path::Path;

/// Returns the base64 of a path whose display form is not byte-exact.
///
/// A path is an OS byte string and JSON is not. Where the two agree the display
/// form is the whole answer and a sibling field would discriminate nothing;
/// where they do not, the display form has already lost bytes.
pub(crate) fn lossy_bytes(path: &Path) -> Option<String> {
    use std::os::unix::ffi::OsStrExt as _;
    let raw = path.as_os_str().as_bytes();
    if path.to_str().is_some() {
        return None;
    }
    Some(base64(raw))
}

/// Encodes bytes as standard, padded base64.
pub(crate) fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let mut buffer = [0_u8; 3];
        buffer[..chunk.len()].copy_from_slice(chunk);
        let packed = u32::from(buffer[0]) << 16 | u32::from(buffer[1]) << 8 | u32::from(buffer[2]);
        for index in 0..4 {
            if index <= chunk.len() {
                let shift = 18 - index * 6;
                out.push(char::from(ALPHABET[((packed >> shift) & 0x3F) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Adds the base64 sibling of a lossy path, or leaves the field absent.
///
/// The sibling's name is the display field's own name with `_b64` appended, so
/// a reader never has to learn a second naming rule.
pub(crate) fn with_lossy_sibling(target: &mut serde_json::Value, field: &str, path: &Path) {
    if let Some(encoded) = lossy_bytes(path) {
        target[format!("{field}_b64")] = serde_json::Value::String(encoded);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn a_utf8_path_carries_no_base64_sibling() {
        assert_eq!(lossy_bytes(Path::new("/c/settings/base.json")), None);
    }

    #[test]
    fn a_non_utf8_path_carries_its_raw_bytes() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt as _};
        let path = PathBuf::from(OsString::from_vec(vec![b'/', b'p', 0x80]));
        assert_eq!(lossy_bytes(&path), Some("L3CA".to_owned()));
    }

    #[test]
    fn base64_pads_every_partial_group() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(&b"foobar"[..2]), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn a_utf8_path_adds_no_sibling_field() {
        let mut value = serde_json::json!({ "path": "/c/base.json" });
        with_lossy_sibling(&mut value, "path", Path::new("/c/base.json"));
        assert!(value.get("path_b64").is_none());
    }
}
