//! The composed-settings input digest and the entry names derived from it.
//!
//! This module is for the pure function from a profile's inputs to one entry
//! name. It is not for reading files, which is `services::storage::entry`, and
//! not for producing the settings document those names hold.

use std::{
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use super::identifier::Identifier;

/// The literal tag that versions the preimage format.
///
/// Changing the sidecar's shape bumps this, which renames every entry, which
/// makes an old sidecar unreachable rather than misread.
const DOMAIN_TAG: &[u8] = b"claude-session-composed-v1";

/// How many hexadecimal characters name an entry.
const SHORT: usize = 12;

/// One piece's contribution to the digest.
#[derive(Clone, Debug)]
pub(crate) struct PieceDigest {
    /// The piece's resolved absolute path.
    pub(crate) path: PathBuf,
    /// The SHA-256 of the piece's bytes.
    pub(crate) content: [u8; 32],
}

/// Everything the entry key is a function of.
///
/// Nothing about the terminal, the working directory, or the account is here,
/// and that absence is the design: two profiles launched from one terminal name
/// two entries, and identical inputs from two terminals name one.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EntryInputs<'a> {
    /// The profile name.
    pub(crate) profile: &'a Identifier,
    /// The profile file's resolved absolute path.
    pub(crate) profile_path: &'a Path,
    /// The SHA-256 of the profile file's bytes.
    pub(crate) profile_content: [u8; 32],
    /// The pieces, in profile order.
    pub(crate) pieces: &'a [PieceDigest],
}

/// Computes the full input digest.
pub(crate) fn input_digest(inputs: &EntryInputs<'_>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    field(&mut hasher, DOMAIN_TAG);
    field(&mut hasher, inputs.profile.as_str().as_bytes());
    field(&mut hasher, inputs.profile_path.as_os_str().as_bytes());
    field(&mut hasher, &inputs.profile_content);
    for piece in inputs.pieces {
        field(&mut hasher, piece.path.as_os_str().as_bytes());
        field(&mut hasher, &piece.content);
    }
    hasher.finalize().into()
}

/// Feeds one length-prefixed field to the hasher.
///
/// The prefix is what makes the preimage unambiguous: without it, two distinct
/// input sets whose concatenations happen to agree would name one entry, which
/// is the property "two profiles never share settings" actually rests on. Eight
/// bytes, big-endian, on every field including the fixed-width digests, so no
/// call site has to decide.
fn field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

/// Renders a digest as 64 lowercase hexadecimal characters.
pub(crate) fn hex(digest: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    digest
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// Returns the settings and provenance paths for one digest.
pub(crate) fn entry_paths(
    composed: &Path,
    profile: &Identifier,
    digest: &[u8; 32],
) -> (PathBuf, PathBuf) {
    // The hex string is sliced, not the digest bytes: twelve hexadecimal
    // characters is not a whole number of bytes.
    let short = &hex(digest)[..SHORT];
    let stem = format!("profile-{}-{short}", profile.as_str());
    (
        composed.join(format!("{stem}.json")),
        composed.join(format!("{stem}.compose.json")),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn identifier(value: &str) -> Identifier {
        value.parse().expect("identifier")
    }

    fn piece(path: &str, byte: u8) -> PieceDigest {
        PieceDigest {
            path: PathBuf::from(path),
            content: [byte; 32],
        }
    }

    fn digest_of(
        profile: &Identifier,
        path: &Path,
        content: u8,
        pieces: &[PieceDigest],
    ) -> [u8; 32] {
        input_digest(&EntryInputs {
            profile,
            profile_path: path,
            profile_content: [content; 32],
            pieces,
        })
    }

    #[test]
    fn identical_inputs_produce_one_digest() {
        let work = identifier("work");
        let path = Path::new("/c/profiles/work.yaml");
        let pieces = [piece("/c/settings/base.json", 1)];
        assert_eq!(
            digest_of(&work, path, 9, &pieces),
            digest_of(&work, path, 9, &pieces)
        );
    }

    #[test]
    fn a_changed_piece_content_names_a_new_entry() {
        let work = identifier("work");
        let path = Path::new("/c/profiles/work.yaml");
        assert_ne!(
            digest_of(&work, path, 9, &[piece("/c/settings/base.json", 1)]),
            digest_of(&work, path, 9, &[piece("/c/settings/base.json", 2)])
        );
    }

    #[test]
    fn a_changed_piece_path_names_a_new_entry() {
        let work = identifier("work");
        let path = Path::new("/c/profiles/work.yaml");
        assert_ne!(
            digest_of(&work, path, 9, &[piece("/c/settings/base.json", 1)]),
            digest_of(&work, path, 9, &[piece("/c/settings/other.json", 1)])
        );
    }

    #[test]
    fn a_reordered_piece_list_names_a_new_entry() {
        let work = identifier("work");
        let path = Path::new("/c/profiles/work.yaml");
        let a = piece("/c/settings/base.json", 1);
        let b = piece("/c/settings/work.json", 2);
        assert_ne!(
            digest_of(&work, path, 9, &[a.clone(), b.clone()]),
            digest_of(&work, path, 9, &[b, a])
        );
    }

    #[test]
    fn a_changed_profile_name_names_a_new_entry() {
        let path = Path::new("/c/profiles/work.yaml");
        let pieces = [piece("/c/settings/base.json", 1)];
        assert_ne!(
            digest_of(&identifier("work"), path, 9, &pieces),
            digest_of(&identifier("home"), path, 9, &pieces)
        );
    }

    /// The strategy table is a field of the profile file, so this is also the
    /// test that a changed strategy names a new entry.
    #[test]
    fn a_changed_profile_content_names_a_new_entry() {
        let work = identifier("work");
        let path = Path::new("/c/profiles/work.yaml");
        let pieces = [piece("/c/settings/base.json", 1)];
        assert_ne!(
            digest_of(&work, path, 9, &pieces),
            digest_of(&work, path, 8, &pieces)
        );
    }

    /// The decisive test. Two input sets whose unframed concatenation would be
    /// byte-identical must still differ. It rejects an implementation that
    /// feeds the fields to the hasher without a length prefix — which passes
    /// every other test in this module, because they all vary a field's
    /// content rather than a field's boundary.
    #[test]
    fn field_framing_resists_boundary_imitation() {
        let work = identifier("work");
        let path = Path::new("/c/p.yaml");
        let left = [piece("/a/bc", 1), piece("/d", 1)];
        let right = [piece("/a/b", 1), piece("/cd", 1)];
        assert_ne!(
            digest_of(&work, path, 0, &left),
            digest_of(&work, path, 0, &right)
        );
    }

    /// A path is a byte string, so a path the display form would mangle still
    /// has to reach the hasher intact.
    #[test]
    fn a_non_utf8_path_is_hashed_as_raw_bytes() {
        use std::os::unix::ffi::OsStringExt;
        let work = identifier("work");
        let path = Path::new("/c/p.yaml");
        let odd = PathBuf::from(OsString::from_vec(vec![b'/', b'p', 0x80]));
        let other = PathBuf::from(OsString::from_vec(vec![b'/', b'p', 0x81]));
        let left = [PieceDigest {
            path: odd,
            content: [1; 32],
        }];
        let right = [PieceDigest {
            path: other,
            content: [1; 32],
        }];
        assert_ne!(
            digest_of(&work, path, 0, &left),
            digest_of(&work, path, 0, &right)
        );
    }

    #[test]
    fn an_entry_is_named_by_twelve_lowercase_hex_characters() {
        let work = identifier("work");
        let digest = digest_of(&work, Path::new("/c/p.yaml"), 0, &[]);
        let full = hex(&digest);
        assert_eq!(full.len(), 64);
        assert!(
            full.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
        let (settings, provenance) = entry_paths(Path::new("/s/composed"), &work, &digest);
        let short = &full[..SHORT];
        assert_eq!(
            settings,
            PathBuf::from(format!("/s/composed/profile-work-{short}.json"))
        );
        assert_eq!(
            provenance,
            PathBuf::from(format!("/s/composed/profile-work-{short}.compose.json"))
        );
    }
}
