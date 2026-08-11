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

/// Command-line inputs to configuration resolution.
///
/// Extracted before the configuration layers exist, because `--config` decides
/// what configuration is.
#[derive(Clone, Debug, Default)]
pub(crate) struct ConfigOverrides {
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
    Version {
        mode: OutputMode,
    },
    Doctor {
        mode: OutputMode,
        list: bool,
        strict: bool,
    },
}

/// Fully classified wrapper invocation.
#[derive(Debug)]
pub(crate) struct Invocation {
    config_overrides: ConfigOverrides,
    verbose: u8,
    quiet: bool,
    kind: InvocationKind,
}

impl Invocation {
    /// Returns the command line's contribution to configuration resolution.
    pub(crate) const fn config_overrides(&self) -> &ConfigOverrides {
        &self.config_overrides
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
            InvocationKind::Version { mode } | InvocationKind::Doctor { mode, .. } => mode,
            _ => OutputMode::Human,
        }
    }
    /// Reports whether this request only projects static catalog metadata.
    pub(crate) const fn is_doctor_list(&self) -> bool {
        matches!(self.kind, InvocationKind::Doctor { list: true, .. })
    }
    /// Reports whether the wrapper owns this invocation as doctor.
    pub(crate) const fn is_doctor(&self) -> bool {
        matches!(self.kind, InvocationKind::Doctor { .. })
    }
    /// Returns doctor strict policy when this is a doctor request.
    pub(crate) const fn doctor_strict(&self) -> bool {
        match self.kind {
            InvocationKind::Doctor { strict, .. } => strict,
            _ => false,
        }
    }
}

/// A command result ready for entry-point status conversion.
///
/// `Exec` is not a status: it is the launch handed back unperformed, because the
/// replacement has to happen after the log flush the entry point owns
/// ([ADR-0080](../../docs/decisions/ADR-0080-order-the-boundary-as-report-flush-exit.md)).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DispatchOutcome {
    Complete(u8),
    Exec(crate::domain::child::ChildInvocation),
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
        .filter(|value| matches!(*value, "doctor" | "help" | "version"));
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
    let config_overrides = ConfigOverrides {
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
            Some(Command::Doctor(value)) => InvocationKind::Doctor {
                mode: if value.json {
                    OutputMode::Json
                } else {
                    OutputMode::Human
                },
                list: value.list,
                strict: value.strict,
            },
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
        config_overrides,
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
        InvocationKind::Doctor { list, strict, .. } => super::doctor::run(context, list, strict),
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
