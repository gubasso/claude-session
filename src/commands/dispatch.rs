//! Pure classification followed by application dispatch.

use std::{ffi::OsString, path::PathBuf};

use clap::Parser;

use crate::{
    cli::{Cli, Command},
    context::AppContext,
    domain::{argv, identifier::Identifier},
    error::{AppError, Diagnostic},
};

/// Terminal output format owned by the active wrapper verb.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutputMode {
    Human,
    Json,
}

/// Parsed global values needed before full configuration loading.
#[derive(Clone, Debug, Default)]
pub(crate) struct Globals {
    /// Explicit user configuration replacement.
    pub(crate) config: Option<PathBuf>,
    /// Invocation account override.
    pub(crate) account: Option<Identifier>,
    /// Invocation profile override.
    pub(crate) profile: Option<Identifier>,
}

/// One of the three slice-001 invocation classes.
#[derive(Debug)]
pub(crate) enum InvocationKind {
    Passthrough(Vec<OsString>),
    Help,
    Version { json: bool },
}

/// Fully classified wrapper invocation.
#[derive(Debug)]
pub(crate) struct Invocation {
    globals: Globals,
    verbose: u8,
    quiet: bool,
    kind: InvocationKind,
}

impl Invocation {
    /// Returns configuration-producing CLI globals.
    pub(crate) const fn globals(&self) -> &Globals {
        &self.globals
    }
    /// Returns the clamped verbosity count.
    pub(crate) const fn verbosity(&self) -> u8 {
        if self.verbose > 3 { 3 } else { self.verbose }
    }
    /// Returns the active output mode.
    pub(crate) const fn output_mode(&self) -> OutputMode {
        match self.kind {
            InvocationKind::Version { json: true } => OutputMode::Json,
            _ => OutputMode::Human,
        }
    }
}

/// A command result ready for entry-point status conversion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DispatchOutcome {
    Complete(u8),
    Signaled(i32),
}

/// Pre-splits raw arguments and parses only wrapper-owned bytes.
pub(crate) fn classify(arguments: &[OsString]) -> Result<Invocation, AppError> {
    let partition = argv::split(arguments)?;
    let (mut wrapper, mut child) = partition.into_parts();
    let wrapper_verb = child
        .first()
        .and_then(|value| value.to_str())
        .filter(|value| matches!(*value, "help" | "version"));
    if wrapper_verb.is_some() {
        wrapper.append(&mut child);
    }
    let cli = Cli::try_parse_from(&wrapper).map_err(|error| {
        AppError::Usage(Diagnostic::new(
            "invalid command line",
            "wrapper arguments",
            error.to_string(),
            "run claude-session --help",
        ))
    })?;
    let globals = Globals {
        config: cli.config,
        account: cli.account,
        profile: cli.profile,
    };
    let kind = if cli.help_flag {
        InvocationKind::Help
    } else if cli.version_flag {
        InvocationKind::Version { json: false }
    } else {
        match cli.command {
            Some(Command::Help(_)) => InvocationKind::Help,
            Some(Command::Version(value)) => InvocationKind::Version { json: value.json },
            None => InvocationKind::Passthrough(child),
        }
    };
    Ok(Invocation {
        globals,
        verbose: cli.verbose,
        quiet: cli.quiet,
        kind,
    })
}

/// Routes one classified invocation without reparsing.
pub(crate) fn dispatch(
    context: &AppContext,
    invocation: Invocation,
) -> Result<DispatchOutcome, AppError> {
    if invocation.quiet {
        tracing::trace!(op = "dispatch", status = "ok", "quiet mode selected");
    }
    let _output_mode = context.output_mode();
    let _paths = context.paths();
    match invocation.kind {
        InvocationKind::Passthrough(arguments) => super::passthrough::run(context, arguments),
        InvocationKind::Help => super::help::run(context),
        InvocationKind::Version { json } => super::version::run(context, json),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn denylist_membership_matches_parser() {
        let command = <Cli as clap::CommandFactory>::command();
        let mut spellings = Vec::new();
        for argument in command.get_arguments() {
            if let Some(long) = argument.get_long() {
                spellings.push(format!("--{long}"));
            }
            if let Some(short) = argument.get_short() {
                spellings.push(format!("-{short}"));
            }
        }
        spellings.sort();
        assert_eq!(
            spellings,
            [
                "--account",
                "--config",
                "--help",
                "--profile",
                "--quiet",
                "--verbose",
                "--version",
                "-V",
                "-h",
                "-q"
            ]
        );
    }

    #[test]
    fn parser_errors_are_usage() {
        let error = classify(&["w".into(), "--config".into()]).expect_err("missing value");
        assert_eq!(error.exit_code(), 64);
    }
}
