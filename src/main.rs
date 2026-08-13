//! The process boundary for `claude-session`.
//!
//! Two things live here and nothing else: the connection to process globals,
//! and the conversion of a failure into a rendered diagnostic plus a status.
//! The fallible program is `run`, which takes what it needs as parameters and
//! reads no global — so the argument grammar can be exercised without a
//! process, and "the arguments were parsed" is a fact of the signature rather
//! than a convention. The order of the conversion — report, flush, then exit —
//! is fixed by [ADR-0080](../docs/decisions/ADR-0080-order-the-boundary-as-report-flush-exit.md).
// ADR-0007 keeps layer boundaries crate-visible in this binary-only crate.
#![allow(clippy::redundant_pub_crate)]

mod adapters;
mod cli;
mod commands;
mod config;
mod context;
mod domain;
mod error;
mod logging;
mod services;
mod ui;
mod util;

use std::process::ExitCode;

use adapters::environment::{Environment, SystemEnvironment};
use commands::dispatch::{DispatchOutcome, Invocation, OutputMode};
use context::AppContext;
use domain::paths::XdgPaths;
use error::AppError;

/// Everything resolved before a subscriber can exist.
///
/// Logging needs a verbosity and a state namespace, and both are themselves
/// results of classifying argv. That is why the spine has two halves.
struct Prepared {
    environment: SystemEnvironment,
    invocation: Invocation,
    paths: XdgPaths,
}

fn main() -> ExitCode {
    let environment = SystemEnvironment::capture();
    let invocation = match commands::dispatch::classify(environment.args()) {
        Ok(value) => value,
        Err(error) => return ExitCode::from(ui::writer::report(&error, OutputMode::Human)),
    };
    if invocation.is_doctor_list() {
        let writer = crate::ui::writer::OutputWriter::system();
        let color = crate::ui::writer::Color::resolve(
            environment.variables(),
            invocation.output_mode(),
            writer.stdout_is_terminal(),
            writer.stderr_is_terminal(),
        );
        let code =
            crate::ui::doctor::list(&writer, invocation.output_mode() == OutputMode::Json, color)
                .map_or_else(
                    |error| ui::writer::report(&error, invocation.output_mode()),
                    |()| 0,
                );
        return ExitCode::from(code);
    }
    let doctor = invocation.is_doctor();
    let mode = invocation.output_mode();
    let strict = invocation.doctor_strict();
    let prepared = match prepare(environment.clone(), invocation) {
        Ok(value) => value,
        // No namespace is known yet, so this one failure reaches standard error
        // and nothing else. ADR-0080 records why that is a boundary condition
        // rather than a gap.
        Err(error) if doctor => {
            return ExitCode::from(commands::doctor::without_paths(
                &environment,
                mode,
                strict,
                &error,
            ));
        }
        Err(error) => return ExitCode::from(ui::writer::report(&error, OutputMode::Human)),
    };
    let logging = logging::install(
        &prepared.paths,
        prepared.invocation.verbosity(),
        mode,
        prepared.environment.variables(),
    );
    // The rest is the conversion ADR-0080 orders: report while the sink is
    // still alive, flush it, then exit or exec. It stays here, in one piece,
    // because splitting it puts the flush and the two things it separates in
    // three places a reader has to hold at once.
    let ending = match run(prepared) {
        Ok(value) => value,
        Err(error) => DispatchOutcome::Complete(ui::writer::report(&error, mode)),
    };
    finish(
        ending,
        move || drop(logging),
        |invocation| {
            use adapters::process::ProcessRunner as _;
            let error = adapters::process::SystemProcessRunner.exec(invocation);
            ui::writer::report(&error, mode)
        },
    )
}

/// Runs the boundary's last two steps in the order [ADR-0080] fixes.
///
/// The flush and the launch arrive as parameters for one reason: an exec does not
/// return, so nothing observable afterwards can prove the flush preceded it. Here
/// the order is a property of one function that a test calls in-process, instead
/// of a race a real run wins nearly every time and loses in exactly the run a
/// reader needs the log for.
///
/// `launch` reports its own failure and returns the code, because by the time it
/// runs the sink is gone and standard error is the only channel left.
///
/// [ADR-0080]: ../docs/decisions/ADR-0080-order-the-boundary-as-report-flush-exit.md
fn finish<F, L>(ending: DispatchOutcome, flush: F, launch: L) -> ExitCode
where
    F: FnOnce(),
    L: FnOnce(&domain::child::ChildInvocation) -> u8,
{
    flush();
    match ending {
        DispatchOutcome::Complete(code) => ExitCode::from(code),
        // The last thing the wrapper does is stop being the wrapper.
        DispatchOutcome::Exec(invocation) => ExitCode::from(launch(&invocation)),
    }
}

/// Classifies the invocation and resolves the namespaces it will write to.
///
/// Separate from `main` only because `?` is illegal in a function returning
/// `ExitCode`. Folding it back in would trade one match for two.
fn prepare(environment: SystemEnvironment, invocation: Invocation) -> Result<Prepared, AppError> {
    let paths = XdgPaths::resolve(environment.variables())?;
    Ok(Prepared {
        environment,
        invocation,
        paths,
    })
}

/// The fallible program. Reads no process global.
fn run(prepared: Prepared) -> Result<DispatchOutcome, AppError> {
    let Prepared {
        environment,
        invocation,
        paths,
    } = prepared;
    let config = config::load::resolve(&environment, &paths, invocation.config_overrides());
    let mode = invocation.output_mode();
    let context = match config {
        Ok(resolution) => {
            let selection = services::account::resolve_selection(&resolution.config, &paths)?;
            AppContext::new(resolution, selection, paths, environment, mode)
        }
        Err(error) if invocation.reports_config_defects() => AppContext::with_config_error(
            config::load::Resolution {
                config: domain::config::ResolvedConfig::defaults(),
                consulted: Vec::new(),
            },
            domain::account::AccountSelection::none(),
            paths,
            environment,
            mode,
            error.into(),
        ),
        Err(error) => return Err(error.into()),
    };
    commands::dispatch::dispatch(&context, invocation)
}

#[cfg(test)]
mod tests {
    use super::{DispatchOutcome, ExitCode, domain::child::ChildInvocation, finish};
    use std::{cell::RefCell, path::PathBuf};

    /// What the boundary did, in the order it did it.
    fn record(ending: DispatchOutcome, code: u8) -> (Vec<&'static str>, ExitCode) {
        let steps = RefCell::new(Vec::new());
        let exit = finish(
            ending,
            || steps.borrow_mut().push("flush"),
            |_| {
                steps.borrow_mut().push("launch");
                code
            },
        );
        (steps.into_inner(), exit)
    }

    fn launch() -> DispatchOutcome {
        DispatchOutcome::Exec(ChildInvocation::new(
            PathBuf::from("/usr/bin/claude"),
            Vec::new(),
            Vec::new(),
        ))
    }

    /// The whole of the flush-before-exec contract, and the only place it can be
    /// observed: after a real exec there is no process left to assert in, and a
    /// log file written by a worker thread races the replacement rather than
    /// ordering against it.
    #[test]
    fn the_flush_precedes_the_launch() {
        let (steps, exit) = record(launch(), 71);
        assert_eq!(steps, ["flush", "launch"]);
        assert_eq!(format!("{exit:?}"), format!("{:?}", ExitCode::from(71)));
    }

    /// A completed verb still flushes, and never reaches the launch.
    #[test]
    fn a_completed_verb_flushes_and_does_not_launch() {
        let (steps, exit) = record(DispatchOutcome::Complete(3), 0);
        assert_eq!(steps, ["flush"]);
        assert_eq!(format!("{exit:?}"), format!("{:?}", ExitCode::from(3)));
    }
}
