//! The write-once composed-settings pair: inspect, decide, and write.
//!
//! This module is for naming a profile's entry from its inputs and reaching one
//! of the four states the owner page defines — reused, written, replaced, or
//! refused. The merge itself belongs to `domain::merge`; what lives here is the
//! filesystem half.
//!
//! Inspection and materialization are deliberately separate functions.
//! `inspect` reads, hashes, composes in memory, and stats the pair; it never
//! creates a directory, because `config` has to be able to describe an entry
//! that does not exist without bringing it into being.

use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

use crate::{
    adapters::filesystem::{FileSystem, SystemFileSystem},
    context::AppContext,
    domain::{
        checks::{Check, CheckResult, EntryCheck},
        encoding::with_lossy_sibling,
        entry::{EntryInputs, PieceDigest, entry_paths, hex, input_digest},
        identifier::Identifier,
        merge::{Composed, MergeError},
        profile::Profile,
        strategy::StrategyTable,
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

/// Everything one read-only inspection learned about a profile's entry.
///
/// Built from one pass over the inputs, so the key, the content, and the
/// provenance always describe the same snapshot: reading an input twice would
/// let a mid-run edit make the three disagree.
#[allow(
    dead_code,
    reason = "the report's full shape is consumed by the composition verbs"
)]
#[derive(Clone, Debug)]
pub(crate) struct EntryReport {
    /// The settings document's path.
    pub(crate) settings: PathBuf,
    /// The provenance sidecar's path.
    pub(crate) provenance: PathBuf,
    /// The full input digest, as 64 lowercase hexadecimal characters.
    pub(crate) digest: String,
    /// The profile document's resolved absolute path.
    pub(crate) profile_path: PathBuf,
    /// The ordered pieces and their digests.
    pub(crate) pieces: Vec<(Identifier, PieceDigest)>,
    /// The strategy table the profile declared.
    pub(crate) strategies: StrategyTable,
    /// The composed document and its per-key provenance.
    pub(crate) composed: Composed,
    /// Whether the settings member is already on disk.
    pub(crate) settings_exists: bool,
    /// Whether the provenance member is already on disk.
    pub(crate) provenance_exists: bool,
}

/// Reports the two entry checks without materialising anything.
///
/// Takes the inspection rather than performing one, so a caller that also
/// renders the profile from it cannot describe two different snapshots of
/// inputs the user may be editing underneath the command.
pub(crate) fn doctor_results(
    profile: &Identifier,
    inspected: &Result<EntryReport, AppError>,
) -> Vec<CheckResult> {
    let compose_check = Check::Entry(EntryCheck::Compose);
    let consistent_check = Check::Entry(EntryCheck::Consistent);
    let report = match *inspected {
        Ok(ref value) => value,
        Err(ref error) => {
            // Existence is this check's whole subject. A profile that exists but
            // is unusable belongs to `settings-profile-valid`, and reporting it
            // here too would make one failure look like two.
            let compose = if error.kind() == ErrorKind::NoInput {
                CheckResult::defect(
                    compose_check,
                    error.diagnostic().why.clone(),
                    error.diagnostic().hint.clone(),
                )
            } else {
                CheckResult::skipped(
                    compose_check,
                    concat!(
                        "the profile document itself is unusable, so what it names could not ",
                        "be looked for. See the profile document check below."
                    ),
                )
            };
            return vec![
                compose,
                CheckResult::skipped(
                    consistent_check,
                    concat!(
                        "settings composition produced nothing to compare, because the ",
                        "profile above could not be used."
                    ),
                ),
            ];
        }
    };
    let compose = CheckResult::pass(
        compose_check,
        format!(
            "profile \"{}\" names pieces that all exist",
            profile.as_str()
        ),
    );
    let consistency = match (report.settings_exists, report.provenance_exists) {
        (false, false) => CheckResult::skipped(
            consistent_check,
            concat!(
                "nothing has been composed yet. The first launch with this profile writes ",
                "the entry, and this check has something to compare from then on."
            ),
        ),
        (true, true)
            if recorded_digest(&report.provenance)
                .ok()
                .flatten()
                .as_deref()
                == Some(&report.digest) =>
        {
            CheckResult::pass(
                consistent_check,
                format!(
                    "{} still matches the pieces it was built from",
                    report.settings.display()
                ),
            )
        }
        _ => CheckResult::defect(
            consistent_check,
            format!(
                concat!(
                    "{} is partial, malformed, or no longer matches the pieces it was ",
                    "built from."
                ),
                report.settings.display()
            ),
            consistent_check
                .hint(&[
                    ("path", &report.settings.display().to_string()),
                    ("profile", profile.as_str()),
                ])
                .unwrap_or_default(),
        ),
    };
    vec![compose, consistency]
}

/// Reports whether the selected profile document is structurally usable.
///
/// Its own catalog entry rather than a leg of `settings-compose`, because that
/// check's published "passes when" is about existence and its remediation says
/// to create the file — advice that is wrong for a file that is already there
/// ([ADR-0018](../../../docs/decisions/ADR-0018-make-every-prerequisite-a-catalog-entry.md)).
pub(crate) fn validity_result(
    context: &AppContext,
    profile: Option<&Identifier>,
    inspected: Option<&Result<EntryReport, AppError>>,
) -> CheckResult {
    let check = Check::Entry(EntryCheck::Valid);
    let (Some(profile), Some(inspected)) = (profile, inspected) else {
        return CheckResult::skipped(
            check,
            concat!(
                "no profile is selected. Bind one to the account with: ",
                "claude-session account bind <account> --profile <name>"
            ),
        );
    };
    let path = context.paths().profile_file(profile);
    match *inspected {
        Ok(_) => CheckResult::pass(
            check,
            format!(
                "profile \"{}\" and the merge rules it declares are usable",
                profile.as_str()
            ),
        ),
        // A missing input is `settings-compose`'s subject, and an unreadable one
        // says nothing about whether the document is valid.
        Err(ref error) if error.kind() != ErrorKind::DataFormat => {
            CheckResult::skipped(check, error.diagnostic().why.clone())
        }
        Err(ref error) => CheckResult::defect(
            check,
            error.diagnostic().why.clone(),
            check
                .hint(&[
                    ("path", &path.display().to_string()),
                    ("profile", profile.as_str()),
                ])
                .unwrap_or_default(),
        ),
    }
}

/// Reads, hashes, and composes one profile's inputs without writing anything.
///
/// Read-only by construction: it never calls `guard::ensure_directory` or
/// `atomic`, so a report command can describe an entry that does not exist
/// without bringing the store into being.
pub(crate) fn inspect(context: &AppContext, profile: &Identifier) -> Result<EntryReport, AppError> {
    let paths = context.paths();
    let profile_path = paths.profile_file(profile);
    let profile_bytes = read_input(&profile_path, profile, EntryCheck::Compose)?;
    // A profile that exists but does not parse is a different failure from one
    // that is missing: `Compose` covers existence, `Valid` covers usability, and
    // `exit-codes.md` publishes `DataFormat` for the second.
    let document = Profile::parse(&String::from_utf8_lossy(&profile_bytes)).map_err(|error| {
        AppError::new(
            EntryCheck::Valid.kind(),
            EntryCheck::Valid.diagnostic(&profile_path, profile.as_str(), &error.to_string()),
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
    let composed = crate::domain::merge::compose(&bodies, document.strategies())
        .map_err(|error| merge_error(&error, profile, &profile_path, &pieces))?;
    let canonical_profile = canonical(context, &profile_path)?;
    let digest = input_digest(&EntryInputs {
        profile,
        profile_path: &canonical_profile,
        profile_content: sha256(&profile_bytes),
        pieces: &pieces
            .iter()
            .map(|(_, piece)| piece.clone())
            .collect::<Vec<_>>(),
    });
    let full = hex(&digest);
    let (settings, provenance) = entry_paths(&paths.composed(), profile, &digest);
    let settings_exists = SystemFileSystem::look(&settings)
        .map_err(|error| io_error(&settings, &error))?
        .is_some();
    let provenance_exists = SystemFileSystem::look(&provenance)
        .map_err(|error| io_error(&provenance, &error))?
        .is_some();
    Ok(EntryReport {
        settings,
        provenance,
        digest: full,
        profile_path: canonical_profile,
        pieces,
        strategies: document.strategies().clone(),
        composed,
        settings_exists,
        provenance_exists,
    })
}

/// Renders a merge failure as the four-part diagnostic.
///
/// `Where` is a concrete resolved path, `Why` names the pointer and the pieces,
/// and `Hint` is the catalog remediation verbatim, so `config` and `doctor`
/// cannot quote two different instructions for one failure.
fn merge_error(
    error: &MergeError,
    profile: &Identifier,
    profile_path: &Path,
    pieces: &[(Identifier, PieceDigest)],
) -> AppError {
    let name = |index: usize| {
        pieces
            .get(index)
            .map_or_else(|| format!("piece {index}"), |(name, _)| name.to_string())
    };
    let where_path = |index: usize| {
        pieces.get(index).map_or_else(
            || profile_path.display().to_string(),
            |(_, piece)| piece.path.display().to_string(),
        )
    };
    let (location, why) = match *error {
        MergeError::NotAnObject { piece } => (
            where_path(piece),
            format!("the piece {} is not a JSON object at its root", name(piece)),
        ),
        MergeError::TypeConflict {
            ref pointer,
            earlier,
            later,
            earlier_type,
            later_type,
        } => (
            where_path(later),
            format!(
                "{pointer} is a {earlier_type} in piece {} and a {later_type} in piece {}",
                name(earlier),
                name(later)
            ),
        ),
        MergeError::StrategyTargetMissing { ref pointer } => (
            profile_path.display().to_string(),
            format!("the array strategy for {pointer} matches no key in the composed document"),
        ),
        MergeError::StrategyTargetNotAnArray {
            ref pointer,
            actual_type,
        } => (
            profile_path.display().to_string(),
            format!("the array strategy for {pointer} names a {actual_type} rather than an array"),
        ),
        MergeError::DuplicateMergeKey {
            ref pointer,
            ref key,
            ref value,
            piece,
        } => (
            where_path(piece),
            format!(
                "two elements of {pointer} share the merge key {key} value {value} in piece {}",
                name(piece)
            ),
        ),
        MergeError::MergeKeyMissing {
            ref pointer,
            ref key,
            piece,
        } => (
            where_path(piece),
            format!(
                "an element of {pointer} in piece {} carries no {key} to merge on",
                name(piece)
            ),
        ),
    };
    let mut diagnostic = EntryCheck::Valid.diagnostic(profile_path, profile.as_str(), &why);
    diagnostic.where_ = location;
    AppError::new(EntryCheck::Valid.kind(), diagnostic)
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

    let report = inspect(context, profile)?;
    let outcome = materialise(paths.state(), profile, &report)?;

    tracing::debug!(
        op = "compose_settings",
        profile = profile.as_str(),
        entry = %report.settings.display(),
        digest = &report.digest[..12],
        outcome = match outcome {
            Outcome::Reused => "reused",
            Outcome::Written => "written",
            Outcome::Replaced => "replaced",
        },
        status = "ok",
        "resolved the composed settings entry"
    );

    Ok(ResolvedEntry {
        settings: report.settings,
        provenance: report.provenance,
        digest: report.digest,
        outcome,
    })
}

/// Decides among the owner page's four cases and writes when one of them says to.
fn materialise(
    state: &Path,
    profile: &Identifier,
    report: &EntryReport,
) -> Result<Outcome, AppError> {
    let settings = report.settings.as_path();
    let provenance = report.provenance.as_path();
    // Re-stat rather than trusting the inspection's snapshot: the decision below
    // is about what is on disk at the moment of writing.
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
        if recorded_digest(provenance)?.as_deref() == Some(&report.digest) {
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
    atomic::write(settings, &serialize(&report.composed.document)?, 0o600)?;
    atomic::write(provenance, &sidecar(profile, report)?, 0o600)?;
    Ok(outcome)
}

/// Serializes the composed document.
///
/// One form, chosen once. The bytes are what the child reads and what the
/// immutability tests compare, so changing the rendering without bumping the
/// domain tag would rewrite every entry under its existing name.
fn serialize(document: &serde_json::Value) -> Result<Vec<u8>, AppError> {
    let mut bytes = serde_json::to_vec_pretty(document).map_err(|error| {
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

/// Renders the provenance sidecar.
///
/// There is no version field: the digest preimage's domain tag versions the
/// format, so changing this shape bumps the tag, which renames every entry and
/// makes an old sidecar unreachable rather than misread.
fn sidecar(profile: &Identifier, report: &EntryReport) -> Result<Vec<u8>, AppError> {
    let mut document = serde_json::json!({
        "profile": profile.as_str(),
        "profile_path": report.profile_path.display().to_string(),
        "digest": report.digest.as_str(),
    });
    with_lossy_sibling(&mut document, "profile_path", &report.profile_path);
    let entries: Vec<serde_json::Value> = report
        .pieces
        .iter()
        .map(|(name, piece)| {
            let mut entry = serde_json::json!({
                "name": name.as_str(),
                "path": piece.path.display().to_string(),
            });
            with_lossy_sibling(&mut entry, "path", &piece.path);
            entry
        })
        .collect();
    document["pieces"] = serde_json::Value::Array(entries);
    document["keys"] = contributor_map(report);
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

/// Renders the per-key contributor map, in pointer order.
///
/// `piece` is always present. Everything else is omitted when it would say
/// nothing: a single-contributor key stays one line, because an absent optional
/// field is omitted rather than emitted as null or an empty list
/// (`logging-and-output.md`).
fn contributor_map(report: &EntryReport) -> serde_json::Value {
    let name = |index: usize| {
        report
            .pieces
            .get(index)
            .map_or_else(|| index.to_string(), |(name, _)| name.as_str().to_owned())
    };
    let mut map = serde_json::Map::new();
    for (pointer, provenance) in &report.composed.keys {
        let mut entry = serde_json::json!({ "piece": name(provenance.piece) });
        if !provenance.overrode.is_empty() {
            entry["overrode"] = provenance
                .overrode
                .iter()
                .map(|index| serde_json::Value::String(name(*index)))
                .collect();
        }
        if let Some(strategy) = provenance.strategy.as_ref() {
            entry["strategy"] = serde_json::Value::String(strategy.spelling().to_owned());
            entry["contributors"] = provenance
                .contributors
                .iter()
                .map(|index| serde_json::Value::String(name(*index)))
                .collect();
        }
        map.insert(pointer.as_str().to_owned(), entry);
    }
    serde_json::Value::Object(map)
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

    /// The fold's own behaviour is `domain::merge`'s to prove; what this module
    /// owns is the byte rendering, which the entry key depends on staying fixed.
    #[test]
    fn one_document_serializes_to_byte_identical_output() {
        let document = value(r#"{"a":1,"b":{"c":2}}"#);
        assert_eq!(
            serialize(&document).expect("serializes"),
            serialize(&document).expect("serializes")
        );
    }

    #[test]
    fn the_serialized_document_ends_with_exactly_one_newline() {
        let bytes = serialize(&value(r#"{"a":1}"#)).expect("serializes");
        assert!(bytes.ends_with(b"}\n"));
        assert!(!bytes.ends_with(b"\n\n"));
    }
}
