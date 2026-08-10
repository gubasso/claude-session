//! The write-once composed-settings pair: resolve, decide, and write.
//!
//! This module is for naming a profile's entry from its inputs and reaching one
//! of the four states the owner page defines — reused, written, replaced, or
//! refused. It is not for the strategy table, structural validation, or the
//! sidecar's contributor map, which belong with the composition verbs.

use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

use crate::{
    adapters::filesystem::{FileSystem, SystemFileSystem},
    context::AppContext,
    domain::{
        checks::EntryCheck,
        entry::{EntryInputs, PieceDigest, entry_paths, hex, input_digest},
        identifier::Identifier,
        profile::Profile,
    },
    error::{AppError, Diagnostic, ErrorKind},
};

use super::{atomic, guard};

/// Why the run did or did not write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Outcome {
    /// The pair was complete and its sidecar agreed, so nothing was composed.
    Reused,
    /// Neither member existed, so both were written.
    Written,
    /// Exactly one member existed, so both were written from this run's inputs.
    Replaced,
}

/// What one run resolved about a profile's composed entry.
///
/// The launch reads `settings` and nothing else; the other three are what the
/// composition verbs report, and they arrive with those verbs.
#[allow(
    dead_code,
    reason = "provenance, digest, and outcome are reported by the composition verbs"
)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedEntry {
    /// The settings document's path.
    pub(crate) settings: PathBuf,
    /// The provenance sidecar's path.
    pub(crate) provenance: PathBuf,
    /// The full input digest, as 64 lowercase hexadecimal characters.
    pub(crate) digest: String,
    /// What this run did.
    pub(crate) outcome: Outcome,
}

/// Resolves, and if necessary materialises, one profile's composed entry.
pub(crate) fn resolve(
    context: &AppContext,
    profile: &Identifier,
) -> Result<ResolvedEntry, AppError> {
    let paths = context.paths();
    let store = paths.composed();
    guard::ensure_directory(paths.state(), &store)?;
    atomic::sweep(&store);

    // The config base is user-authored and `0644` by design, so it carries no
    // security check and is read without a guard walk.
    let profile_path = paths.profile_file(profile);
    let profile_bytes = read_input(&profile_path, profile, EntryCheck::Compose)?;
    let document = Profile::parse(&String::from_utf8_lossy(&profile_bytes)).map_err(|error| {
        AppError::new(
            ErrorKind::DataFormat,
            Diagnostic::new(
                "the profile document is invalid",
                profile_path.display().to_string(),
                error.to_string(),
                "correct the profile: it needs a non-empty `layers` list and no unknown keys",
            ),
        )
    })?;

    let mut pieces = Vec::with_capacity(document.layers().len());
    let mut bodies = Vec::with_capacity(document.layers().len());
    for name in document.layers() {
        let path = paths.piece_file(name);
        let bytes = read_input(&path, profile, EntryCheck::Compose)?;
        bodies.push(parse_piece(&path, &bytes)?);
        pieces.push((
            name.clone(),
            PieceDigest {
                path: canonical(context, &path)?,
                content: sha256(&bytes),
            },
        ));
    }

    let digests: Vec<PieceDigest> = pieces.iter().map(|(_, piece)| piece.clone()).collect();
    let canonical_profile = canonical(context, &profile_path)?;
    let digest = input_digest(&EntryInputs {
        profile,
        profile_path: &canonical_profile,
        profile_content: sha256(&profile_bytes),
        pieces: &digests,
    });
    let full = hex(&digest);
    let (settings, provenance) = entry_paths(&store, profile, &digest);

    let outcome = materialise(
        paths.state(),
        &settings,
        &provenance,
        profile,
        &canonical_profile,
        &pieces,
        &full,
        &bodies,
    )?;

    tracing::debug!(
        op = "compose_settings",
        profile = profile.as_str(),
        entry = %settings.display(),
        digest = &full[..12],
        outcome = match outcome {
            Outcome::Reused => "reused",
            Outcome::Written => "written",
            Outcome::Replaced => "replaced",
        },
        status = "ok",
        "resolved the composed settings entry"
    );

    Ok(ResolvedEntry {
        settings,
        provenance,
        digest: full,
        outcome,
    })
}

/// Decides among the owner page's four cases and writes when one of them says to.
#[allow(clippy::too_many_arguments)] // Every argument is one input of the decision.
fn materialise(
    state: &Path,
    settings: &Path,
    provenance: &Path,
    profile: &Identifier,
    profile_path: &Path,
    pieces: &[(Identifier, PieceDigest)],
    digest: &str,
    bodies: &[serde_json::Value],
) -> Result<Outcome, AppError> {
    let has_settings = SystemFileSystem::look(settings)
        .map_err(|error| io_error(settings, &error))?
        .is_some();
    let has_provenance = SystemFileSystem::look(provenance)
        .map_err(|error| io_error(provenance, &error))?
        .is_some();

    if has_settings && has_provenance {
        guard::validate(state, settings, guard::Expected::PrivateFile)?;
        guard::validate(state, provenance, guard::Expected::PrivateFile)?;
        // The comparison costs nothing: the inputs were already read to compute
        // the key. It is what makes "two profiles never share settings" a check
        // rather than a probability.
        if recorded_digest(provenance)?.as_deref() == Some(digest) {
            return Ok(Outcome::Reused);
        }
        return Err(AppError::new(
            EntryCheck::Consistent.kind(),
            EntryCheck::Consistent.diagnostic(
                settings,
                profile.as_str(),
                "the sidecar's recorded digest disagrees with the one its inputs recompute",
            ),
        ));
    }

    // Exactly one member present is an unverifiable entry: settings without
    // provenance carry no digest to compare, so adopting the survivor would
    // turn the guarantee above back into a probability. Both members are
    // written from this run's inputs, replacing whichever survived.
    //
    // A survivor is still a wrapper-managed leaf, so it faces the same checks
    // a complete pair does before this run replaces it. Replacing is writing:
    // `docs/reference/xdg-storage.md#filesystem-security` applies the link,
    // ownership, and type checks to every managed path, and a rename over an
    // unchecked leaf would destroy a foreign artifact the wrapper is required
    // to refuse.
    let outcome = if has_settings || has_provenance {
        if has_settings {
            guard::validate(state, settings, guard::Expected::PrivateFile)?;
        }
        if has_provenance {
            guard::validate(state, provenance, guard::Expected::PrivateFile)?;
        }
        Outcome::Replaced
    } else {
        Outcome::Written
    };
    atomic::write(settings, &compose(bodies)?, 0o600)?;
    atomic::write(
        provenance,
        &sidecar(profile, profile_path, pieces, digest)?,
        0o600,
    )?;
    Ok(outcome)
}

/// Folds the pieces left to right into one document.
///
/// The default fold and nothing more: objects merge key by key, scalars take
/// the last writer, and an array replaces. That is exactly what
/// `docs/reference/configuration.md#merge-semantics` defines as the behaviour of
/// an array no strategy table lists, so implementing it here settles nothing
/// the strategy work has not already inherited.
fn compose(bodies: &[serde_json::Value]) -> Result<Vec<u8>, AppError> {
    let mut folded = serde_json::Value::Object(serde_json::Map::new());
    for body in bodies {
        merge(&mut folded, body);
    }
    let mut bytes = serde_json::to_vec_pretty(&folded).map_err(|error| {
        AppError::new(
            ErrorKind::Internal,
            Diagnostic::new(
                "the composed document could not be serialized",
                "composed settings",
                error.to_string(),
                "report this: composition produces a document it just built",
            ),
        )
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn merge(into: &mut serde_json::Value, from: &serde_json::Value) {
    match (into, from) {
        (serde_json::Value::Object(target), serde_json::Value::Object(source)) => {
            for (key, value) in source {
                match target.get_mut(key) {
                    Some(existing) => merge(existing, value),
                    None => {
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (target, source) => *target = source.clone(),
    }
}

/// Renders the provenance sidecar.
///
/// The `keys` contributor map is deliberately absent: it is a by-product of the
/// merge machinery that resolves per-key strategies, and an empty one would
/// claim an answer this fold cannot give. There is no version field either —
/// the digest preimage's domain tag versions the format.
fn sidecar(
    profile: &Identifier,
    profile_path: &Path,
    pieces: &[(Identifier, PieceDigest)],
    digest: &str,
) -> Result<Vec<u8>, AppError> {
    let mut document = serde_json::json!({
        "profile": profile.as_str(),
        "profile_path": profile_path.display().to_string(),
        "digest": digest,
    });
    if let Some(encoded) = lossy_bytes(profile_path) {
        document["profile_path_b64"] = serde_json::Value::String(encoded);
    }
    let entries: Vec<serde_json::Value> = pieces
        .iter()
        .map(|(name, piece)| {
            let mut entry = serde_json::json!({
                "name": name.as_str(),
                "path": piece.path.display().to_string(),
            });
            if let Some(encoded) = lossy_bytes(&piece.path) {
                entry["path_b64"] = serde_json::Value::String(encoded);
            }
            entry
        })
        .collect();
    document["pieces"] = serde_json::Value::Array(entries);
    let mut bytes = serde_json::to_vec_pretty(&document).map_err(|error| {
        AppError::new(
            ErrorKind::Internal,
            Diagnostic::new(
                "the provenance sidecar could not be serialized",
                "composition provenance",
                error.to_string(),
                "report this: provenance is built from values already in hand",
            ),
        )
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Returns the base64 of a path whose display form is not byte-exact.
///
/// A path is an OS byte string and JSON is not. Where the two agree the display
/// form is the whole answer and a sibling field would discriminate nothing;
/// where they do not, the display form has already lost bytes.
fn lossy_bytes(path: &Path) -> Option<String> {
    use std::os::unix::ffi::OsStrExt as _;
    let raw = path.as_os_str().as_bytes();
    if path.to_str().is_some() {
        return None;
    }
    Some(base64(raw))
}

/// Encodes bytes as standard, padded base64.
fn base64(bytes: &[u8]) -> String {
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

/// Reads the `digest` field a sidecar records.
fn recorded_digest(provenance: &Path) -> Result<Option<String>, AppError> {
    use std::io::Read as _;
    let mut handle = SystemFileSystem::open_private_file(provenance)
        .map_err(|error| io_error(provenance, &error))?;
    let mut bytes = Vec::new();
    handle
        .read_to_end(&mut bytes)
        .map_err(|error| io_error(provenance, &error))?;
    let document: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        AppError::new(
            EntryCheck::Consistent.kind(),
            EntryCheck::Consistent.diagnostic(provenance, "", &error.to_string()),
        )
    })?;
    Ok(document
        .get("digest")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned))
}

fn parse_piece(path: &Path, bytes: &[u8]) -> Result<serde_json::Value, AppError> {
    serde_json::from_slice(bytes).map_err(|error| {
        AppError::new(
            ErrorKind::DataFormat,
            Diagnostic::new(
                "a settings piece is not valid JSON",
                path.display().to_string(),
                error.to_string(),
                "correct the piece: it is a partial settings document in the child's format",
            ),
        )
    })
}

fn read_input(path: &Path, profile: &Identifier, check: EntryCheck) -> Result<Vec<u8>, AppError> {
    match SystemFileSystem::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(AppError::new(
            check.kind(),
            check.diagnostic(path, profile.as_str(), &error.to_string()),
        )),
        Err(error) => Err(io_error(path, &error)),
    }
}

/// Resolves a path to its absolute physical form, as the digest preimage requires.
///
/// Fallible on purpose. `docs/reference/xdg-storage.md#composed-settings-entries`
/// puts the resolved absolute path in the preimage, so substituting the
/// unresolved one would hash a preimage the specification does not describe:
/// the digest could not be reproduced from the algorithm, and one input set
/// could name two entries across runs. A relative path would additionally let
/// the working directory reach the key, which is exactly what
/// [ADR-0064] keys by inputs to prevent.
///
/// [ADR-0064]:
///     ../../../docs/decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md
fn canonical(context: &AppContext, path: &Path) -> Result<PathBuf, AppError> {
    context
        .adapters()
        .filesystem()
        .canonicalize(path)
        .map_err(|error| io_error(path, &error))
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn io_error(path: &Path, error: &std::io::Error) -> AppError {
    AppError::new(
        ErrorKind::Io,
        Diagnostic::new(
            "the composed settings store could not be read",
            path.display().to_string(),
            error.to_string(),
            "check the path and the filesystem",
        ),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn value(text: &str) -> serde_json::Value {
        serde_json::from_str(text).expect("json")
    }

    #[test]
    fn objects_merge_recursively_and_scalars_take_the_last_writer() {
        let bytes = compose(&[
            value(r#"{"model":"a","env":{"X":"1","Y":"2"}}"#),
            value(r#"{"model":"b","env":{"Y":"3"}}"#),
        ])
        .expect("composes");
        let folded: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(folded["model"], "b");
        assert_eq!(folded["env"]["X"], "1");
        assert_eq!(folded["env"]["Y"], "3");
    }

    /// Replace is the default an unlisted array takes, and it is what makes the
    /// fold predictable: what the last piece says is what you get.
    #[test]
    fn an_array_replaces_rather_than_appending() {
        let bytes = compose(&[value(r#"{"allow":["a","b"]}"#), value(r#"{"allow":["c"]}"#)])
            .expect("composes");
        let folded: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(folded["allow"], value(r#"["c"]"#));
    }

    /// The entry is named by its inputs, so the same inputs have to produce the
    /// same bytes — otherwise a rewrite would be invisible to the digest and
    /// visible in the file.
    #[test]
    fn one_input_set_folds_to_byte_identical_output() {
        let bodies = [value(r#"{"b":1,"a":2}"#), value(r#"{"c":3}"#)];
        assert_eq!(
            compose(&bodies).expect("composes"),
            compose(&bodies).expect("composes")
        );
    }

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
}
