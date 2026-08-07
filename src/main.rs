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
    let prepared = match prepare(SystemEnvironment::capture()) {
        Ok(value) => value,
        // No namespace is known yet, so this one failure reaches standard error
        // and nothing else. ADR-0080 records why that is a boundary condition
        // rather than a gap.
        Err(error) => return ExitCode::from(ui::writer::report(&error, OutputMode::Human)),
    };
    let mode = prepared.invocation.output_mode();
    let logging = logging::install(
        &prepared.paths,
        prepared.invocation.verbosity(),
        mode,
        prepared.environment.variables(),
    );
    // The rest is the conversion ADR-0080 orders: report while the sink is
    // still alive, flush it, then exit. It stays here, in one piece, because
    // splitting it puts the flush and the two things it separates in three
    // places a reader has to hold at once.
    let ending = match run(prepared) {
        Ok(value) => value,
        Err(error) => DispatchOutcome::Complete(ui::writer::report(&error, mode)),
    };
    drop(logging);
    match ending {
        DispatchOutcome::Complete(code) => ExitCode::from(code),
        // Dying of the child's signal is what makes a parent see a real
        // `WIFSIGNALED`. Re-raising does not return; the `128 + N` encoding is
        // the fallback for when it does, and is what a shell would report.
        DispatchOutcome::Signaled(signal) => {
            let _ = signal_hook::low_level::emulate_default_handler(signal);
            ExitCode::from(
                u8::try_from(128_i32.saturating_add(signal).clamp(0, 255)).unwrap_or(255),
            )
        }
    }
}

/// Classifies the invocation and resolves the namespaces it will write to.
///
/// Separate from `main` only because `?` is illegal in a function returning
/// `ExitCode`. Folding it back in would trade one match for two.
fn prepare(environment: SystemEnvironment) -> Result<Prepared, AppError> {
    let invocation = commands::dispatch::classify(environment.args())?;
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
    let config = config::load::resolve(&environment, &paths, invocation.config_overrides())?;
    let mode = invocation.output_mode();
    let context = AppContext::new(config, paths, environment, mode);
    commands::dispatch::dispatch(&context, invocation)
}
