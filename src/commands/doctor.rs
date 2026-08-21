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
                "XDG_RUNTIME_DIR is not set, so this session has no runtime directory.",
                String::new(),
            )
        },
        |path| {
            CheckResult::pass(
                Check::RuntimeDirPresent,
                std::path::Path::new(path).display().to_string(),
            )
        },
    ));
    for check in &CATALOG[2..] {
        results.push(CheckResult::skipped(
            *check,
            concat!(
                "the wrapper could not work out where to store its files, so nothing ",
                "below it could be checked. Fix the storage locations above first."
            ),
        ));
    }
    let child = ChildReport::skipped("the wrapper could not work out where to store its files");
    let summary = Summary::fold(&results);
    let verdict = Verdict::fold(summary, &child, strict);
    let writer = crate::ui::writer::OutputWriter::system();
    let color = crate::ui::writer::Color::resolve(
        environment.variables(),
        mode,
        writer.stdout_is_terminal(),
        writer.stderr_is_terminal(),
    );
    crate::ui::doctor::report(
        &writer,
        mode == OutputMode::Json,
        color,
        &results,
        summary,
        &child,
        verdict,
    )
    .map_or_else(
        |write_error| crate::ui::writer::report(&write_error, mode, color),
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
        crate::ui::doctor::list(
            context.writer(),
            context.output_mode() == OutputMode::Json,
            context.color(),
        )?;
        return Ok(DispatchOutcome::Complete(0));
    }
    let mut results = Vec::with_capacity(CATALOG.len());
    results.push(CheckResult::pass(
        Check::BaseDirsResolve,
        format!(
            "settings in {}, state in {}",
            context.paths().config().display(),
            context.paths().state().display()
        ),
    ));
    let runtime = environment::value(context.environment().variables(), "XDG_RUNTIME_DIR");
    results.push(runtime.map_or_else(
        || {
            CheckResult::defect(
                Check::RuntimeDirPresent,
                "XDG_RUNTIME_DIR is not set, so this session has no runtime directory.",
                String::new(),
            )
        },
        |path| {
            CheckResult::pass(
                Check::RuntimeDirPresent,
                std::path::Path::new(path).display().to_string(),
            )
        },
    ));
    results.push(context.config_error().map_or_else(
        || {
            CheckResult::pass(
                Check::WrapperConfigParses,
                "read, with every key recognized",
            )
        },
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
                path.display().to_string(),
            ));
            results.push(CheckResult::pass(
                Check::ChildIsExecutable,
                "this user can run it",
            ));
            Some(path.clone())
        }
        Err(error) if error.kind() == ErrorKind::ChildNotExecutable => {
            results.push(CheckResult::pass(
                Check::ChildBinaryResolves,
                error.diagnostic().where_.clone(),
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
                "there is no claude to test. Install it first, as the check above says.",
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
            concat!(
                "claude could not be run, so it could not be asked for its version. ",
                "Fix the checks above first."
            ),
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
                concat!(
                    "the wrapper configuration could not be read, so no session could be ",
                    "worked out from it. Fix the configuration above first."
                ),
            ));
        }
    } else if has_session {
        use crate::services::storage::guard::{Expected, ProbeTarget};
        // Each entry is a path and, for a declared link, the target it must
        // resolve to. Both are owned here so the borrows below outlive the
        // probe.
        let mut paths: Vec<(std::path::PathBuf, Option<std::path::PathBuf>)> = Vec::new();
        if let Some(account) = context.session().account() {
            paths.push((account.directory.clone(), None));
            paths.push((account.config.clone(), None));
            // The session directory this run is inside, when there is one. A
            // launch's own directory does not exist until the launch makes it,
            // so the only session there is to inspect is the agent this command
            // is running under ([ADR-0113]).
            if let Ok(agent) = crate::services::session::agent(context) {
                let space = agent.namespace().id();
                paths.push((context.paths().account_namespace(&account.id, space), None));
            }
            if let Some(session) = crate::services::session::current(context, &account.id) {
                paths.push((session.clone(), None));
                paths.push((
                    session.join("projects"),
                    Some(context.paths().account_projects(&account.id)),
                ));
                // The shared peer registry and its link, probed only when this
                // run's scope derives: an underivable scope leaves the session
                // unshared by design, which is not a storage defect
                // (ADR-0108, share the child peer registry across sessions).
                if let Some(scope) = crate::adapters::host::boot_id().and_then(|boot| {
                    crate::adapters::host::namespace_link(crate::domain::namespace::Kind::Mount)
                        .and_then(|link| crate::domain::peers::Scope::derive(&boot, &link))
                }) {
                    let registry = context
                        .paths()
                        .peer_registry(scope.boot(), scope.namespace());
                    let link = session.join("sessions");
                    // A real directory at the name is the pre-adoption state
                    // the next launch converts, not a defect worth reporting;
                    // the probe judges the name once it is absent or linked.
                    let adopted = !matches!(
                        crate::adapters::filesystem::SystemFileSystem::look(&link),
                        Ok(Some(facts)) if facts.directory
                    );
                    if adopted {
                        paths.push((link, Some(registry.clone())));
                    }
                    paths.push((registry, None));
                }
                // Every asset seat a launch would inspect is a declared link,
                // and a check promising that declared links resolve has to look
                // at all of them. Every declared name rather than the ones the
                // tree currently holds: a launch validates the seat whether or
                // not the source is still there, so filtering by presence would
                // leave the report silent about exactly the link an emptied
                // tree makes the next launch refuse. A seat that does not exist
                // produces no defect, because the walk stops at the first
                // component that is not there.
                let tree = context.paths().assets();
                for name in crate::services::assets::declared() {
                    paths.push((session.join(name), Some(tree.join(name))));
                }
            }
        }
        if context.session().profile().is_some() {
            paths.push((context.paths().composed(), None));
        }
        let targets: Vec<ProbeTarget<'_>> = paths
            .iter()
            .map(|(path, target)| ProbeTarget {
                path,
                expected: target
                    .as_ref()
                    .map_or(Expected::Directory, |value| Expected::DeclaredLink(value)),
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
            results.push(CheckResult::skipped(
                *check,
                concat!(
                    "no account or profile is selected, so this run touches none of the ",
                    "wrapper's session files."
                ),
            ));
        }
    }
    if context.config_error().is_none() {
        // One inspection feeds both entry checks and the validity check below,
        // so the three cannot describe different snapshots of inputs the user
        // may be editing while doctor runs.
        let inspected = context
            .session()
            .profile()
            .map(|profile| crate::services::storage::entry::inspect(context, profile));
        if let (Some(profile), Some(inspected)) = (context.session().profile(), inspected.as_ref())
        {
            results.extend(crate::services::storage::entry::doctor_results(
                profile, inspected,
            ));
        } else {
            for check in [
                Check::Entry(crate::domain::checks::EntryCheck::Compose),
                Check::Entry(crate::domain::checks::EntryCheck::Consistent),
            ] {
                results.push(CheckResult::skipped(
                    check,
                    concat!(
                        "no profile is selected. Bind one to the account with: ",
                        "claude-session account bind <account> --profile <name>"
                    ),
                ));
            }
        }
        results.extend(crate::services::account::doctor_results(context));
        // After the account pair, because the pushed order has to match the
        // catalog's and each of these was appended after it.
        results.push(crate::services::storage::entry::validity_result(
            context,
            context.session().profile(),
            inspected.as_ref(),
        ));
        results.push(crate::services::account::launch_ready_result(context));
        results.push(crate::services::account::plan_declared_result(context));
        // Last, because it was appended to the catalog last and the pushed
        // order has to match it.
        results.push(identity_result(context));
        results.push(assets_result(context));
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
        context.color(),
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
            // Two conditions, not one: a version that is genuinely old, and
            // output no version could be read from. Folding them into one
            // sentence is what let a current child be reported as below the
            // floor ([ADR-0095]).
            match ChildVersion::parse(&captured.stdout) {
                Some(version) if version >= MINIMUM_CHILD_VERSION => CheckResult::pass(
                    check,
                    format!("{version}, which meets the {MINIMUM_CHILD_VERSION} minimum"),
                ),
                Some(version) => CheckResult::defect(
                    check,
                    format!(
                        concat!(
                            "claude reports {observed}, older than the {minimum} this ",
                            "wrapper is designed against."
                        ),
                        observed = version,
                        minimum = MINIMUM_CHILD_VERSION
                    ),
                    floor_hint(check),
                ),
                None => CheckResult::defect(
                    check,
                    format!(
                        concat!(
                            "claude answered \"{observed}\", which carries no version ",
                            "number this wrapper could read, so the {minimum} minimum is ",
                            "unconfirmed."
                        ),
                        observed = observed,
                        minimum = MINIMUM_CHILD_VERSION
                    ),
                    floor_hint(check),
                ),
            }
        }
        Ok(_) => CheckResult::defect(
            check,
            concat!(
                "claude --version did not exit successfully, so the minimum version is ",
                "unconfirmed."
            )
            .to_owned(),
            floor_hint(check),
        ),
        Err(error) => CheckResult::defect(check, error.diagnostic().why.clone(), floor_hint(check)),
    }
}

/// Returns the one remediation the floor owns, true of every way it can fail.
fn floor_hint(check: Check) -> String {
    check
        .hint(&[("minimum", &MINIMUM_CHILD_VERSION.to_string())])
        .unwrap_or_default()
}

/// Reports the agent a launch from this run would name.
///
/// A refusal is the check failing rather than the run failing: `doctor` exists
/// to say what a launch would meet, and a launch is where the refusal belongs.
fn identity_result(context: &AppContext) -> CheckResult {
    match crate::services::session::agent(context) {
        Ok(agent) => CheckResult::pass(
            Check::SessionIdentityDerives,
            format!(
                "a launch from this run would be \"{}\" in {} \"{}\", discriminated by {}",
                agent.id().as_str(),
                agent.namespace().kind().as_str(),
                agent.namespace().id().as_str(),
                agent.namespace().discriminated_by().as_str()
            ),
        ),
        Err(error) => CheckResult::defect(
            Check::SessionIdentityDerives,
            error.diagnostic().why.clone(),
            error.diagnostic().hint.clone(),
        ),
    }
}

/// Reports how many of the child's asset names the user's tree would supply.
///
/// A tree with nothing in it is a warning rather than a failure: it costs the
/// reader every skill they wrote, but it never stops a launch, and a first run
/// on a new machine legitimately has none ([ADR-0106]).
///
/// [ADR-0106]: ../../docs/decisions/ADR-0106-supply-child-assets-from-one-tree.md
fn assets_result(context: &AppContext) -> CheckResult {
    let tree = context.paths().assets();
    let hint = || {
        Check::SessionAssetsLinked
            .hint(&[("path", &tree.display().to_string())])
            .unwrap_or_default()
    };
    let held = match crate::services::assets::present(context) {
        // A tree the wrapper could not look at is not a tree holding nothing,
        // and telling the reader to populate it would be the wrong instruction
        // for a permission they have to fix first.
        crate::services::assets::Survey::Uninspectable(path, why) => {
            return CheckResult::defect(
                Check::SessionAssetsLinked,
                format!("{} could not be inspected: {why}.", path.display()),
                hint(),
            );
        }
        crate::services::assets::Survey::Held(held) => held,
    };
    if held.is_empty() {
        return CheckResult::defect(
            Check::SessionAssetsLinked,
            format!(
                concat!(
                    "{} holds none of the {} assets claude reads, so every session ",
                    "starts without your own."
                ),
                tree.display(),
                crate::services::assets::declared().len()
            ),
            hint(),
        );
    }
    CheckResult::pass(
        Check::SessionAssetsLinked,
        format!(
            "{} would reach every session, from {}",
            held.join(", "),
            tree.display()
        ),
    )
}

fn child_report(
    context: &AppContext,
    program: Option<&std::path::Path>,
) -> Result<(ChildReport, Option<AppError>), AppError> {
    let Some(program) = program else {
        return Ok((
            ChildReport::skipped("claude could not be found or could not be run"),
            None,
        ));
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
