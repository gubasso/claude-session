//! Whether the child's credential file holds a saved login.
//!
//! The child keeps its claude.ai login and every MCP server sign-in in one
//! file, `.credentials.json`. A sign-in to an MCP server creates that file in an
//! account that never logged in to claude.ai, so a file at that path is not
//! evidence of a saved login ([ADR-0128]).
//!
//! The wrapper therefore reads the names of the file's top-level keys, and
//! nothing else. Every value is skipped as the parser passes it, so no token
//! is kept, compared, or fingerprinted, and nothing here writes to the file.
//!
//! [ADR-0128]: ../../../docs/decisions/ADR-0128-tell-a-saved-login-by-its-key.md

use std::{fmt, fs, io::Read as _, os::unix::fs::MetadataExt as _, path::Path};

use rustix::fs::{Mode, OFlags};
use serde::{
    Deserialize, Deserializer,
    de::{IgnoredAny, MapAccess, SeqAccess, Visitor},
};

/// The most the wrapper reads before it gives up on the file.
///
/// The child lets the MCP half grow with every sign-in, and one upstream report
/// shows it near a megabyte. A file past this bound answers "no saved login",
/// which is the answer that never claims something the wrapper did not see.
const LIMIT: u64 = 16 * 1024 * 1024;

/// What the top level of the child's credential file holds.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct CredentialKeys {
    /// `claudeAiOauth` holds an object: the child has a saved login.
    pub(crate) login: bool,
    /// `mcpOAuth` holds at least one entry: the file also carries MCP sign-ins.
    pub(crate) mcp_sign_ins: bool,
}

/// Reads which halves the credential file at `path` holds.
///
/// A missing, linked, foreign, oversized, or malformed file holds neither,
/// because that is what the child itself makes of it.
pub(crate) fn read(path: &Path) -> CredentialKeys {
    probe(path).unwrap_or_default()
}

/// True when the credential file at `path` holds a saved login.
pub(crate) fn present(path: &Path) -> bool {
    read(path).login
}

fn probe(path: &Path) -> Option<CredentialKeys> {
    // `O_NOFOLLOW` refuses a link at the leaf, and the owner and type are taken
    // from the open handle, so nothing swapped in after the check is what gets
    // read. The file is never opened for writing and its mode is never changed.
    let descriptor = rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .ok()?;
    let handle = fs::File::from(descriptor);
    let metadata = handle.metadata().ok()?;
    if !metadata.is_file() || metadata.uid() != rustix::process::getuid().as_raw() {
        return None;
    }
    let mut bytes = Vec::new();
    handle.take(LIMIT + 1).read_to_end(&mut bytes).ok()?;
    if u64::try_from(bytes.len()).ok()? > LIMIT {
        return None;
    }
    let keys: Keys = serde_json::from_slice(&bytes).ok()?;
    Some(CredentialKeys {
        login: keys.login.is_some_and(|value| value.object),
        mcp_sign_ins: keys
            .mcp
            .is_some_and(|value| value.object && value.entries > 0),
    })
}

#[derive(Deserialize)]
struct Keys {
    #[serde(rename = "claudeAiOauth", default)]
    login: Option<Shape>,
    #[serde(rename = "mcpOAuth", default)]
    mcp: Option<Shape>,
}

/// The shape of one value, with its contents skipped.
struct Shape {
    object: bool,
    entries: usize,
}

impl Shape {
    const SCALAR: Self = Self {
        object: false,
        entries: 0,
    };
}

impl<'de> Deserialize<'de> for Shape {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ShapeVisitor)
    }
}

struct ShapeVisitor;

impl<'de> Visitor<'de> for ShapeVisitor {
    type Value = Shape;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Shape, A::Error> {
        let mut entries = 0;
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {
            entries += 1;
        }
        Ok(Shape {
            object: true,
            entries,
        })
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Shape, A::Error> {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(Shape::SCALAR)
    }

    fn visit_bool<E>(self, _: bool) -> Result<Shape, E> {
        Ok(Shape::SCALAR)
    }

    fn visit_i64<E>(self, _: i64) -> Result<Shape, E> {
        Ok(Shape::SCALAR)
    }

    fn visit_u64<E>(self, _: u64) -> Result<Shape, E> {
        Ok(Shape::SCALAR)
    }

    fn visit_f64<E>(self, _: f64) -> Result<Shape, E> {
        Ok(Shape::SCALAR)
    }

    fn visit_str<E>(self, _: &str) -> Result<Shape, E> {
        Ok(Shape::SCALAR)
    }

    fn visit_unit<E>(self) -> Result<Shape, E> {
        Ok(Shape::SCALAR)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::{fs, os::unix::fs::symlink};

    use super::{CredentialKeys, LIMIT, present, read};

    fn file(contents: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join(".credentials.json");
        fs::write(&path, contents).expect("fixture");
        (directory, path)
    }

    #[test]
    fn a_login_object_is_a_saved_login() {
        let (_directory, path) = file(br#"{"claudeAiOauth":{"accessToken":"sk-ant-oat01-x"}}"#);
        assert_eq!(
            read(&path),
            CredentialKeys {
                login: true,
                mcp_sign_ins: false
            }
        );
    }

    #[test]
    fn mcp_sign_ins_alone_are_not_a_saved_login() {
        let (_directory, path) =
            file(br#"{"mcpOAuth":{"firecrawl|0":{"accessToken":"secret","expiresAt":1}}}"#);
        assert_eq!(
            read(&path),
            CredentialKeys {
                login: false,
                mcp_sign_ins: true
            }
        );
    }

    #[test]
    fn both_halves_are_reported_together() {
        let (_directory, path) = file(br#"{"mcpOAuth":{"a":{}},"claudeAiOauth":{}}"#);
        assert_eq!(
            read(&path),
            CredentialKeys {
                login: true,
                mcp_sign_ins: true
            }
        );
    }

    #[test]
    fn an_empty_mcp_object_carries_no_sign_ins() {
        let (_directory, path) = file(br#"{"mcpOAuth":{}}"#);
        assert!(!read(&path).mcp_sign_ins);
    }

    #[test]
    fn a_login_key_that_is_not_an_object_is_no_saved_login() {
        for contents in [
            br#"{"claudeAiOauth":null}"#.as_slice(),
            br#"{"claudeAiOauth":"token"}"#,
            br#"{"claudeAiOauth":[{}]}"#,
            br#"{"claudeAiOauth":7}"#,
        ] {
            let (_directory, path) = file(contents);
            assert!(!present(&path), "{}", String::from_utf8_lossy(contents));
        }
    }

    #[test]
    fn a_malformed_file_is_no_saved_login() {
        for contents in [
            b"child-owned-bytes".as_slice(),
            b"",
            br#"{"claudeAiOauth":{}"#,
            br"[]",
        ] {
            let (_directory, path) = file(contents);
            assert!(!present(&path), "{}", String::from_utf8_lossy(contents));
        }
    }

    #[test]
    fn a_missing_file_is_no_saved_login() {
        let directory = tempfile::tempdir().expect("temporary directory");
        assert!(!present(&directory.path().join(".credentials.json")));
    }

    #[test]
    fn a_link_is_not_followed() {
        let (directory, target) = file(br#"{"claudeAiOauth":{}}"#);
        let link = directory.path().join("link.json");
        symlink(&target, &link).expect("link");
        assert!(present(&target), "fixture");
        assert!(!present(&link));
    }

    #[test]
    fn a_file_past_the_limit_is_no_saved_login() {
        let mut contents = br#"{"claudeAiOauth":{},"pad":""#.to_vec();
        contents.resize(usize::try_from(LIMIT).expect("limit fits"), b' ');
        contents.extend_from_slice(br#""}"#);
        let (_directory, path) = file(&contents);
        assert!(!present(&path));
    }
}
