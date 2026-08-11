//! Non-aborting traversal and exit policy for the doctor report.

use crate::{
    adapters::{
        environment::{self, Environment},
        process::ProcessRunner as _,
    },
    commands::dispatch::{DispatchOutcome, OutputMode},
    context::AppContext,
    domain::{
        checks::{CATALOG, Check, CheckResult, ChildReport, ChildStatus, Summary, Verdict},
        child::{ChildOutcome, ChildVersion, MINIMUM_CHILD_VERSION},
    },
    error::{AppError, ErrorKind},
};

/// Renders a complete dependency-aware report when XDG bootstrap failed.
pub(crate) fn without_paths(
    environment: &crate::adapters::environment::SystemEnvironment,
    mode: OutputMode,
    strict: bool,
    error: &AppError,
) -> u8 {
    let mut results = Vec::with_capacity(CATALOG.len());
    results.push(CheckResult::defect(
        Check::BaseDirsResolve,
        error.diagnostic().why.clone(),
        Check::BaseDirsResolve.hint(&[]).unwrap_or_default(),
    ));
    let runtime = environment::value(environment.variables(), "XDG_RUNTIME_DIR");
    results.push(runtime.map_or_else(
        || {
            CheckResult::defect(
                Check::RuntimeDirPresent,
                "XDG_RUNTIME_DIR is absent",
                String::new(),
            )
        },
        |path| {
            CheckResult::pass(
                Check::RuntimeDirPresent,
                format!("{} is present", std::path::Path::new(path).display()),
            )
        },
    ));
    for check in &CATALOG[2..] {
        results.push(CheckResult::skipped(
            *check,
            "base directory resolution failed",
        ));
    }
    let child = ChildReport::skipped("base directory resolution failed");
    let summary = Summary::fold(&results);
    let verdict = Verdict::fold(summary, &child, strict);
    let writer = crate::ui::writer::OutputWriter::system();
    crate::ui::doctor::report(
        &writer,
        mode == OutputMode::Json,
        &results,
        summary,
        &child,
        verdict,
    )
    .map_or_else(
        |write_error| crate::ui::writer::report(&write_error, mode),
        |()| verdict.exit,
    )
}

/// Runs every safe probe and renders one report.
#[allow(
    clippy::too_many_lines,
    reason = "the catalog traversal stays visible in its owning command"
)]
pub(crate) fn run(
    context: &AppContext,
    list: bool,
    strict: bool,
) -> Result<DispatchOutcome, AppError> {
    if list {
        crate::ui::doctor::list(context.writer(), context.output_mode() == OutputMode::Json)?;
        return Ok(DispatchOutcome::Complete(0));
    }
    let mut results = Vec::with_capacity(CATALOG.len());
    results.push(CheckResult::pass(
        Check::BaseDirsResolve,
        format!(
            "config={} state={}",
            context.paths().config().display(),
            context.paths().state().display()
        ),
    ));
    let runtime = environment::value(context.environment().variables(), "XDG_RUNTIME_DIR");
    results.push(runtime.map_or_else(
        || {
            CheckResult::defect(
                Check::RuntimeDirPresent,
                "XDG_RUNTIME_DIR is absent",
                String::new(),
            )
        },
        |path| {
            CheckResult::pass(
                Check::RuntimeDirPresent,
                format!("{} is present", std::path::Path::new(path).display()),
            )
        },
    ));
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

    let program = crate::services::child::program(context);
    let runnable = match &program {
        Ok(path) => {
            results.push(CheckResult::pass(
                Check::ChildBinaryResolves,
                format!("resolved {}", path.display()),
            ));
            results.push(CheckResult::pass(
                Check::ChildIsExecutable,
                format!("{} is executable", path.display()),
            ));
            Some(path.clone())
        }
        Err(error) if error.kind() == ErrorKind::ChildNotExecutable => {
            results.push(CheckResult::pass(
                Check::ChildBinaryResolves,
                format!("resolved {}", error.diagnostic().where_),
            ));
            results.push(CheckResult::defect(
                Check::ChildIsExecutable,
                error.diagnostic().why.clone(),
                Check::ChildIsExecutable
                    .hint(&[("path", &error.diagnostic().where_)])
                    .unwrap_or_default(),
            ));
            None
        }
        Err(error) => {
            results.push(CheckResult::defect(
                Check::ChildBinaryResolves,
                error.diagnostic().why.clone(),
                Check::ChildBinaryResolves.hint(&[]).unwrap_or_default(),
            ));
            results.push(CheckResult::skipped(
                Check::ChildIsExecutable,
                "child resolution failed",
            ));
            None
        }
    };

    if let Some(path) = &runnable {
        let invocation = crate::domain::child::ChildInvocation::new(
            path.clone(),
            vec!["--version".into()],
            crate::services::child::subroutine_environment(context),
        );
        let version = context.adapters().process().run_captured(&invocation);
        results.push(version_result(version));
    } else {
        results.push(CheckResult::skipped(
            Check::ChildVersionFloor,
            "the child is not executable",
        ));
    }

    let has_session =
        context.session().account().is_some() || context.session().profile().is_some();
    if context.config_error().is_some() {
        for check in CATALOG
            .iter()
            .filter(|check| check.scope() == crate::domain::checks::Scope::Session)
        {
            results.push(CheckResult::skipped(
                *check,
                "wrapper configuration parsing failed",
            ));
        }
    } else if has_session {
        use crate::services::storage::guard::{Expected, ProbeTarget};
        let mut paths = Vec::new();
        if let Some(account) = context.session().account() {
            paths.push((account.directory.clone(), Expected::Directory));
            paths.push((account.config.clone(), Expected::Directory));
        }
        if context.session().profile().is_some() {
            paths.push((context.paths().composed(), Expected::Directory));
        }
        let targets: Vec<ProbeTarget<'_>> = paths
            .iter()
            .map(|(path, expected)| ProbeTarget {
                path,
                expected: *expected,
            })
            .collect();
        results.extend(crate::services::storage::guard::probe(
            context.paths().state(),
            &targets,
        ));
    } else {
        for check in CATALOG
            .iter()
            .filter(|check| matches!(check, Check::Storage(_)))
        {
            results.push(CheckResult::skipped(*check, "no session context applies"));
        }
    }
    if context.config_error().is_none() {
        if let Some(profile) = context.session().profile() {
            results.extend(crate::services::storage::entry::doctor_results(
                context, profile,
            ));
        } else {
            results.push(CheckResult::skipped(
                Check::Entry(crate::domain::checks::EntryCheck::Compose),
                "no profile is selected",
            ));
            results.push(CheckResult::skipped(
                Check::Entry(crate::domain::checks::EntryCheck::Consistent),
                "no profile is selected",
            ));
        }
        results.extend(crate::services::account::doctor_results(context));
        // After the account pair, because the pushed order has to match the
        // catalog's and `settings-profile-valid` is its newest, last entry.
        results.push(crate::services::storage::entry::validity_result(
            context,
            context.session().profile(),
        ));
    }

    let (child, child_race) = child_report(context, runnable.as_deref())?;
    if let Some(error) = child_race {
        match error.kind() {
            ErrorKind::ChildNotFound => {
                results[3] = CheckResult::defect(
                    Check::ChildBinaryResolves,
                    error.diagnostic().why.clone(),
                    Check::ChildBinaryResolves.hint(&[]).unwrap_or_default(),
                );
            }
            ErrorKind::ChildNotExecutable => {
                results[4] = CheckResult::defect(
                    Check::ChildIsExecutable,
                    error.diagnostic().why.clone(),
                    Check::ChildIsExecutable
                        .hint(&[("path", &error.diagnostic().where_)])
                        .unwrap_or_default(),
                );
            }
            _ => {}
        }
    }
    let summary = Summary::fold(&results);
    let verdict = Verdict::fold(summary, &child, strict);
    crate::ui::doctor::report(
        context.writer(),
        context.output_mode() == OutputMode::Json,
        &results,
        summary,
        &child,
        verdict,
    )?;
    Ok(DispatchOutcome::Complete(verdict.exit))
}

fn version_result(result: Result<crate::domain::child::CapturedChild, AppError>) -> CheckResult {
    let check = Check::ChildVersionFloor;
    match result {
        Ok(captured) if matches!(captured.outcome, ChildOutcome::Exited(0)) => {
            let observed = String::from_utf8_lossy(&captured.stdout).trim().to_owned();
            match ChildVersion::parse(&captured.stdout) {
                Some(version) if version >= MINIMUM_CHILD_VERSION => CheckResult::pass(
                    check,
                    format!("child version {version} meets minimum {MINIMUM_CHILD_VERSION}"),
                ),
                _ => CheckResult::defect(
                    check,
                    format!("child version {observed} is below or could not be parsed"),
                    check
                        .hint(&[
                            ("version", &observed),
                            ("minimum", &MINIMUM_CHILD_VERSION.to_string()),
                        ])
                        .unwrap_or_default(),
                ),
            }
        }
        Ok(captured) => CheckResult::defect(
            check,
            "child --version did not exit successfully",
            check
                .hint(&[
                    ("version", &format!("{:?}", captured.outcome)),
                    ("minimum", &MINIMUM_CHILD_VERSION.to_string()),
                ])
                .unwrap_or_default(),
        ),
        Err(error) => CheckResult::defect(
            check,
            error.diagnostic().why.clone(),
            check
                .hint(&[
                    ("version", "unavailable"),
                    ("minimum", &MINIMUM_CHILD_VERSION.to_string()),
                ])
                .unwrap_or_default(),
        ),
    }
}

fn child_report(
    context: &AppContext,
    program: Option<&std::path::Path>,
) -> Result<(ChildReport, Option<AppError>), AppError> {
    let Some(program) = program else {
        return Ok((ChildReport::skipped("the child is not executable"), None));
    };
    let invocation = crate::domain::child::ChildInvocation::new(
        program.to_path_buf(),
        vec!["doctor".into()],
        crate::services::child::subroutine_environment(context),
    );
    match context.adapters().process().run_captured(&invocation) {
        Ok(captured) => {
            if !captured.stderr.is_empty() {
                context
                    .writer()
                    .stderr(&captured.stderr)
                    .map_err(|error| crate::ui::writer::output_error(&error))?;
            }
            // The child's own level crosses unchanged: it reported a problem,
            // so this run reports one too ([ADR-0085]).
            let (status, exit, reason) = match captured.outcome {
                ChildOutcome::Exited(0) => (ChildStatus::Pass, Some(0), None),
                ChildOutcome::Exited(code) => (ChildStatus::Fail, Some(code), None),
                ChildOutcome::Signaled(signal) => (
                    ChildStatus::Fail,
                    None,
                    Some(format!("claude doctor was terminated by signal {signal}")),
                ),
            };
            Ok((
                ChildReport {
                    status,
                    output: Some(captured.stdout),
                    exit,
                    reason,
                },
                None,
            ))
        }
        Err(error) => {
            let is_race = matches!(
                error.kind(),
                ErrorKind::ChildNotFound | ErrorKind::ChildNotExecutable
            );
            Ok((
                ChildReport {
                    status: if is_race {
                        ChildStatus::Skipped
                    } else {
                        ChildStatus::Fail
                    },
                    output: None,
                    exit: None,
                    reason: Some(error.diagnostic().why.clone()),
                },
                is_race.then_some(error),
            ))
        }
    }
}
