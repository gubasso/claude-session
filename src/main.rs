//! The five-step invocation spine for `claude-session`.
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
use commands::dispatch::{DispatchOutcome, OutputMode};
use context::AppContext;
use error::AppError;

fn main() -> ExitCode {
    let environment = SystemEnvironment::capture();
    let args = environment.args().to_vec();
    let invocation = match commands::dispatch::classify(&args) {
        Ok(value) => value,
        Err(error) => return finish_error(&error, OutputMode::Human),
    };
    let output_mode = invocation.output_mode();
    let paths = match domain::paths::XdgPaths::resolve(environment.variables()) {
        Ok(value) => value,
        Err(error) => return finish_error(&error.into(), output_mode),
    };
    let verbosity = invocation.verbosity();
    let logging = logging::install(&paths, verbosity, environment.variables());
    let _owned_namespaces = (paths.data(), paths.cache());
    let config = match config::load::resolve(&environment, &paths, invocation.globals()) {
        Ok(value) => value,
        Err(error) => return finish_error(&error.into(), output_mode),
    };
    let context = AppContext::new(config, paths, environment, invocation.output_mode());
    let result = commands::dispatch::dispatch(&context, invocation);
    drop(logging);
    match result {
        Ok(DispatchOutcome::Complete(code)) => ExitCode::from(code),
        Ok(DispatchOutcome::Signaled(signal)) => reproduce_signal(signal),
        Err(error) => finish_error(&error, output_mode),
    }
}

fn finish_error(error: &AppError, output_mode: OutputMode) -> ExitCode {
    tracing::error!(
        op = "dispatch",
        status = "err",
        err.kind = error.kind().spelling(),
        "wrapper operation failed"
    );
    let writer = ui::writer::OutputWriter::system();
    match output_mode {
        OutputMode::Human => writer.diagnostic(error),
        OutputMode::Json => writer.diagnostic_json(error),
    }
    ExitCode::from(error.exit_code())
}

fn reproduce_signal(signal: i32) -> ExitCode {
    let _ = signal_hook::low_level::emulate_default_handler(signal);
    let fallback = u8::try_from((128_i32.saturating_add(signal)).clamp(0, 255)).unwrap_or(255);
    ExitCode::from(fallback)
}
