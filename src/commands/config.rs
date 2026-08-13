//! The configuration report verb.
//!
//! An assertion verb: it renders its whole report first and decides the exit
//! afterwards, so standard output always carries the requested result even when
//! the answer is that something is broken.

use std::path::PathBuf;

use crate::{
    commands::dispatch::{DispatchOutcome, OutputMode},
    context::AppContext,
    domain::{
        checks::{Check, CheckResult, CheckStatus, EntryCheck, Severity},
        config::Source,
        identifier::Identifier,
        strategy::ArrayStrategy,
    },
    error::AppError,
};

/// One resolved configuration key and the layer that supplied it.
pub(crate) struct KeyReport {
    /// The key's file spelling.
    pub(crate) name: &'static str,
    /// The resolved value, absent when no layer supplied one.
    pub(crate) value: Option<String>,
    /// The winning layer.
    pub(crate) source: Source,
}

/// One piece a profile names, with the path it resolved to.
pub(crate) struct PieceReport {
    pub(crate) name: Identifier,
    pub(crate) path: PathBuf,
}

/// One declared array strategy.
pub(crate) struct StrategyReport {
    pub(crate) pointer: String,
    pub(crate) strategy: String,
    pub(crate) key: Option<String>,
}

/// The active profile, its inputs, and the entry they name.
pub(crate) struct ProfileReport {
    pub(crate) name: Identifier,
    pub(crate) path: PathBuf,
    pub(crate) pieces: Vec<PieceReport>,
    pub(crate) strategies: Vec<StrategyReport>,
    pub(crate) settings: PathBuf,
    pub(crate) provenance: PathBuf,
    pub(crate) digest: String,
    /// Whether both members of the pair are already written.
    pub(crate) exists: bool,
}

/// Everything `config` reports, in one value both renderers project from.
///
/// One typed value rather than two renderers assembling their own, so the human
/// and JSON forms cannot omit different fields.
pub(crate) struct Report {
    pub(crate) configuration: Vec<KeyReport>,
    pub(crate) files: Vec<crate::config::load::ConsultedFile>,
    /// Absent when no layer resolved a profile name.
    pub(crate) profile: Option<ProfileReport>,
    pub(crate) defects: Vec<CheckResult>,
}

/// The config-scoped subset of the one probe catalog.
///
/// A subset of the catalog rather than a private check list, so `config` and
/// `doctor` cannot disagree about a defect or quote two different remediations
/// ([ADR-0018](../../docs/decisions/ADR-0018-make-every-prerequisite-a-catalog-entry.md)).
const SCOPED: &[Check] = &[
    Check::WrapperConfigParses,
    Check::Entry(EntryCheck::Compose),
    Check::Entry(EntryCheck::Consistent),
    Check::Entry(EntryCheck::Valid),
];

pub(crate) fn run(context: &AppContext) -> Result<DispatchOutcome, AppError> {
    let report = assemble(context);
    crate::ui::config::report(
        context.writer(),
        context.output_mode() == OutputMode::Json,
        context.color(),
        &report,
    )?;
    Ok(DispatchOutcome::Complete(verdict(&report)))
}

/// Folds the defects into an exit the same way `doctor` does.
///
/// The first failing hard check in catalog order, or `0`. An unwritten entry is
/// `0`: `exit-codes.md` makes it advisory, because it is not a question the
/// user asked that went unanswered.
fn verdict(report: &Report) -> u8 {
    report
        .defects
        .iter()
        .find(|result| {
            result.status == CheckStatus::Fail && result.check.severity() == Severity::Hard
        })
        .map_or(0, |result| result.check.kind().exit_code())
}

fn assemble(context: &AppContext) -> Report {
    let config = context.config();
    let configuration = vec![
        KeyReport {
            name: "child_bin",
            value: config.child_bin().map(|path| path.display().to_string()),
            source: config.child_bin_source(),
        },
        KeyReport {
            name: "default_account",
            value: config.account().map(|name| name.as_str().to_owned()),
            source: config.account_source(),
        },
        KeyReport {
            name: "default_profile",
            value: config.profile().map(|name| name.as_str().to_owned()),
            source: config.profile_source(),
        },
    ];

    let selected = context.session().profile().cloned();
    // One inspection, borrowed by the profile report, the warnings, and the
    // defects, so the three cannot describe different snapshots of inputs the
    // user may be editing while the command runs. It never creates the
    // composed store.
    let inspected = selected
        .as_ref()
        .map(|name| crate::services::storage::entry::inspect(context, name));

    let profile = match (selected.as_ref(), inspected.as_ref()) {
        (Some(name), Some(Ok(entry))) => Some(ProfileReport {
            name: name.clone(),
            path: entry.profile_path.clone(),
            pieces: entry
                .pieces
                .iter()
                .map(|(piece, digest)| PieceReport {
                    name: piece.clone(),
                    path: digest.path.clone(),
                })
                .collect(),
            strategies: entry
                .strategies
                .iter()
                .map(|(pointer, strategy)| StrategyReport {
                    pointer: pointer.as_str().to_owned(),
                    strategy: strategy.spelling().to_owned(),
                    key: match *strategy {
                        ArrayStrategy::MergeByKey { ref key } => Some(key.clone()),
                        ArrayStrategy::Concat => None,
                    },
                })
                .collect(),
            settings: entry.settings.clone(),
            provenance: entry.provenance.clone(),
            digest: entry.digest.clone(),
            exists: entry.settings_exists && entry.provenance_exists,
        }),
        // A profile that resolved but cannot be inspected is reported through
        // the defects below; claiming a half-built profile section here would
        // put a guess where a diagnostic belongs.
        _ => None,
    };

    let defects = defects(context, selected.as_ref(), inspected.as_ref());

    Report {
        configuration,
        files: context.consulted_files().to_vec(),
        profile,
        defects,
    }
}

/// Runs the config-scoped catalog subset, in catalog order.
fn defects(
    context: &AppContext,
    profile: Option<&Identifier>,
    inspected: Option<&Result<crate::services::storage::entry::EntryReport, AppError>>,
) -> Vec<CheckResult> {
    let mut results = Vec::with_capacity(SCOPED.len());
    results.push(context.config_error().map_or_else(
        || CheckResult::pass(Check::WrapperConfigParses, "wrapper configuration parsed"),
        |error| {
            CheckResult::defect(
                Check::WrapperConfigParses,
                error.diagnostic().why.clone(),
                error.diagnostic().hint.clone(),
            )
        },
    ));
    match profile {
        None => {
            results.push(CheckResult::skipped(
                Check::Entry(EntryCheck::Compose),
                "no profile is selected",
            ));
            results.push(CheckResult::skipped(
                Check::Entry(EntryCheck::Consistent),
                "no profile is selected",
            ));
        }
        Some(name) => {
            if let Some(inspected) = inspected {
                results.extend(crate::services::storage::entry::doctor_results(
                    name, inspected,
                ));
            }
        }
    }
    results.push(crate::services::storage::entry::validity_result(
        context, profile, inspected,
    ));
    results
}
