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

/// The user's coarse control over the diagnostic mirror.
///
/// One ladder rather than a count beside a flag: `--quiet` with `--verbose` is a
/// usage error, so the pair can never both be set and a type that could hold
/// both would invite a silent precedence rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Verbosity {
    Quiet,
    Default,
    Info,
    Debug,
    Trace,
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
    Version { mode: OutputMode },
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
    /// Returns the diagnostic-mirror ladder, clamping a fourth repeat.
    pub(crate) const fn verbosity(&self) -> Verbosity {
        if self.quiet {
            return Verbosity::Quiet;
        }
        match self.verbose {
            0 => Verbosity::Default,
            1 => Verbosity::Info,
            2 => Verbosity::Debug,
            _ => Verbosity::Trace,
        }
    }
    /// Returns the active output mode.
    pub(crate) const fn output_mode(&self) -> OutputMode {
        match self.kind {
            InvocationKind::Version { mode } => mode,
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
    let partition: argv::Partition = argv::split(arguments)?;
    // `--` is unconditional: everything after it is child territory even when it
    // spells a wrapper verb, so the rescue below only applies to a suffix that
    // no sentinel closed.
    let sentinel: bool = partition.sentinel();
    let (mut wrapper, mut child) = partition.into_parts();
    let wrapper_verb = child
        .first()
        .filter(|_| !sentinel)
        .and_then(|value| value.to_str())
        .filter(|value| matches!(*value, "help" | "version"));
    if wrapper_verb.is_some() {
        wrapper.append(&mut child);
    }
    let cli = Cli::try_parse_from(&wrapper).map_err(|error| {
        AppError::new(
            crate::error::ErrorKind::Usage,
            Diagnostic::new(
                "invalid command line",
                "wrapper arguments",
                error.to_string(),
                "run claude-session --help",
            ),
        )
    })?;
    let globals = Globals {
        config: cli.config,
        account: cli.account,
        profile: cli.profile,
    };
    let kind = if cli.help_flag {
        InvocationKind::Help
    } else if cli.version_flag {
        InvocationKind::Version {
            mode: OutputMode::Human,
        }
    } else {
        match cli.command {
            Some(Command::Help(_)) => InvocationKind::Help,
            Some(Command::Version(value)) => InvocationKind::Version {
                mode: if value.json {
                    OutputMode::Json
                } else {
                    OutputMode::Human
                },
            },
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
    match invocation.kind {
        InvocationKind::Passthrough(arguments) => super::passthrough::run(context, arguments),
        InvocationKind::Help => super::help::run(context),
        InvocationKind::Version { .. } => super::version::run(context),
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
