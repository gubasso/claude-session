//! Pure classification followed by application dispatch.

use std::{ffi::OsString, path::PathBuf};

use clap::Parser;

use crate::{
    cli::{Cli, Command, account::AccountCommand},
    context::AppContext,
    domain::{
        account::{RecordedAt, TokenIngest, TokenSource},
        argv,
        identifier::Identifier,
    },
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
    AccountLogin {
        name: Option<Identifier>,
        token: Option<TokenIngest>,
        mode: OutputMode,
    },
    AccountList {
        mode: OutputMode,
    },
    AccountStatus {
        name: Option<Identifier>,
        mode: OutputMode,
    },
    AccountRemove {
        name: Identifier,
        consented: bool,
        mode: OutputMode,
    },
    Completion {
        shell: clap_complete::Shell,
    },
    Man,
    Config {
        mode: OutputMode,
    },
    Profile {
        mode: OutputMode,
    },
    /// Requested help for one wrapper verb, or one node inside its namespace.
    VerbHelp {
        verb: &'static str,
        subcommand: Option<&'static str>,
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
            InvocationKind::Version { mode }
            | InvocationKind::Doctor { mode, .. }
            | InvocationKind::AccountLogin { mode, .. }
            | InvocationKind::AccountList { mode }
            | InvocationKind::AccountStatus { mode, .. }
            | InvocationKind::AccountRemove { mode, .. }
            | InvocationKind::Config { mode }
            | InvocationKind::Profile { mode } => mode,
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
    /// Reports whether a failed configuration probe is this verb's subject.
    ///
    /// The two assertion verbs are asked whether a property holds
    /// (`exit-codes.md#exit-regimes-by-verb`), so unresolvable configuration is
    /// the answer they exist to render rather than a reason to abandon the
    /// report. Every other invocation still fails at the boundary.
    pub(crate) const fn reports_config_defects(&self) -> bool {
        matches!(
            self.kind,
            InvocationKind::Doctor { .. } | InvocationKind::Config { .. }
        )
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
        .filter(|value| {
            matches!(
                *value,
                "account"
                    | "completion"
                    | "config"
                    | "doctor"
                    | "help"
                    | "man"
                    | "profile"
                    | "version"
            )
        });
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
                "run claude-session-rs --help",
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
        classify_command(cli.command, child)?
    };
    Ok(Invocation {
        config_overrides,
        verbose: cli.verbose,
        quiet: cli.quiet,
        kind,
    })
}

/// Resolves the parsed subcommand, or the absence of one, into an invocation.
///
/// Separate from `classify` so the pre-split and the verb table each stay one
/// readable unit as the verb set grows.
fn classify_command(
    command: Option<Command>,
    child: Vec<OsString>,
) -> Result<InvocationKind, AppError> {
    Ok(match command {
        Some(Command::Account(value)) => classify_account(value)?,
        Some(Command::Completion(value)) => classify_completion(&value)?,
        Some(Command::Config(value)) => {
            classify_report("config", value.help_flag, value.json, |mode| {
                InvocationKind::Config { mode }
            })
        }
        Some(Command::Doctor(value)) => InvocationKind::Doctor {
            mode: if value.json {
                OutputMode::Json
            } else {
                OutputMode::Human
            },
            list: value.list,
            strict: value.strict,
        },
        // `help <verb>` prints exactly what `<verb> --help` prints
        // (`cli-surface.md#help`), so it routes to the same renderer.
        Some(Command::Help(value)) => {
            value
                .verb
                .map_or(InvocationKind::Help, |topic| InvocationKind::VerbHelp {
                    verb: help_topic_node(topic),
                    subcommand: None,
                })
        }
        Some(Command::Man(value)) => {
            if value.help_flag {
                InvocationKind::VerbHelp {
                    verb: "man",
                    subcommand: None,
                }
            } else {
                InvocationKind::Man
            }
        }
        Some(Command::Profile(value)) => {
            classify_report("profile", value.help_flag, value.json, |mode| {
                InvocationKind::Profile { mode }
            })
        }
        Some(Command::Version(value)) => InvocationKind::Version {
            mode: if value.json {
                OutputMode::Json
            } else {
                OutputMode::Human
            },
        },
        None => InvocationKind::Passthrough(child),
    })
}

/// Renders one verb node's own help, the diagnostic a bare verb earns.
///
/// The same parser node answers `<verb> --help`, so the help shown on failure
/// and the help given on request cannot drift apart.
fn node_help_text(verb: &str) -> String {
    let mut root = <Cli as clap::CommandFactory>::command();
    root.build();
    root.find_subcommand_mut(verb).map_or_else(
        || "no subcommand was given".to_owned(),
        |node| node.render_help().to_string(),
    )
}

/// Resolves a report verb whose whole grammar is `--json` and requested help.
///
/// Requested help is checked before the mode, because help is a result the verb
/// never reaches (`cli-surface.md#help`).
fn classify_report(
    verb: &'static str,
    help_flag: bool,
    json: bool,
    kind: impl FnOnce(OutputMode) -> InvocationKind,
) -> InvocationKind {
    if help_flag {
        return InvocationKind::VerbHelp {
            verb,
            subcommand: None,
        };
    }
    kind(if json {
        OutputMode::Json
    } else {
        OutputMode::Human
    })
}

/// Maps a requested-help topic to the parser node that answers it.
const fn help_topic_node(topic: crate::cli::help::HelpTopic) -> &'static str {
    match topic {
        crate::cli::help::HelpTopic::Account => "account",
        crate::cli::help::HelpTopic::Completion => "completion",
        crate::cli::help::HelpTopic::Config => "config",
        crate::cli::help::HelpTopic::Man => "man",
        crate::cli::help::HelpTopic::Profile => "profile",
    }
}

/// Resolves the `completion` verb into one invocation.
fn classify_completion(
    value: &crate::cli::completion::CompletionArgs,
) -> Result<InvocationKind, AppError> {
    if value.help_flag {
        return Ok(InvocationKind::VerbHelp {
            verb: "completion",
            subcommand: None,
        });
    }
    // The shell had to become optional so `completion --help` could parse, so
    // the parser can no longer raise this itself and the same node renders it
    // here. An unrecognized shell is still the parser's own `Usage`, because
    // the value is a closed `ValueEnum`.
    value.shell.map_or_else(
        || {
            Err(AppError::new(
                crate::error::ErrorKind::Usage,
                Diagnostic::new(
                    "completion requires a shell",
                    "completion",
                    node_help_text("completion"),
                    "run claude-session-rs completion --help",
                ),
            ))
        },
        |shell| Ok(InvocationKind::Completion { shell }),
    )
}

/// Resolves the `account` namespace into one invocation.
fn classify_account(value: crate::cli::account::AccountArgs) -> Result<InvocationKind, AppError> {
    let mode = |json: bool| {
        if json {
            OutputMode::Json
        } else {
            OutputMode::Human
        }
    };
    if value.help_flag {
        return Ok(InvocationKind::VerbHelp {
            verb: "account",
            subcommand: match value.command {
                None => None,
                Some(AccountCommand::Login(_)) => Some("login"),
                Some(AccountCommand::List(_)) => Some("list"),
                Some(AccountCommand::Status(_)) => Some("status"),
                Some(AccountCommand::Remove(_)) => Some("remove"),
            },
        });
    }
    match value.command {
        // A namespace verb satisfies no invocation on its own
        // ([ADR-0052](../../docs/decisions/ADR-0052-require-an-explicit-subcommand.md)),
        // and the verb's own help is that diagnostic (`cli-surface.md#help`),
        // which is what makes the closed subcommand set discoverable at the
        // moment the user meets the failure. The parser can no longer raise it
        // itself, because the subcommand had to become optional for
        // `account --help`, so the same parser node renders it here.
        None => Err(AppError::new(
            crate::error::ErrorKind::Usage,
            Diagnostic::new(
                "account requires a subcommand",
                "account",
                node_help_text("account"),
                "run claude-session-rs account --help",
            ),
        )),
        Some(AccountCommand::Login(value)) => {
            let json = value.json;
            Ok(InvocationKind::AccountLogin {
                name: value.name,
                token: token_ingest(value.token, value.stdin, value.minted_at)?,
                mode: mode(json),
            })
        }
        Some(AccountCommand::List(value)) => Ok(InvocationKind::AccountList {
            mode: mode(value.json),
        }),
        Some(AccountCommand::Status(value)) => Ok(InvocationKind::AccountStatus {
            name: value.name,
            mode: mode(value.json),
        }),
        Some(AccountCommand::Remove(value)) => Ok(InvocationKind::AccountRemove {
            name: value.name,
            consented: value.yes,
            mode: mode(value.json),
        }),
    }
}

/// Resolves the token-mode flags into an ingest request, or none.
///
/// The parser already refuses `--stdin` and `--minted-at` without `--token`, so
/// the only validation left is the timestamp's own grammar. It is checked here
/// rather than at ingest because a malformed value is a usage error, and usage
/// errors belong before any side effect rather than after a browser flow.
fn token_ingest(
    token: bool,
    stdin: bool,
    minted_at: Option<String>,
) -> Result<Option<TokenIngest>, AppError> {
    if !token {
        return Ok(None);
    }
    let minted_at = minted_at
        .map(RecordedAt::parse)
        .transpose()
        .map_err(|message| {
            AppError::new(
                crate::error::ErrorKind::Usage,
                Diagnostic::new(
                    "invalid command line",
                    "--minted-at",
                    message,
                    "pass an RFC 3339 UTC time such as 2026-08-11T12:34:56Z",
                ),
            )
        })?;
    Ok(Some(TokenIngest {
        source: if stdin {
            TokenSource::Stdin
        } else {
            TokenSource::Terminal
        },
        minted_at,
    }))
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
        InvocationKind::AccountLogin { name, token, .. } => {
            super::account::login(context, name, token)
        }
        InvocationKind::AccountList { .. } => super::account::list(context),
        InvocationKind::AccountStatus { name, .. } => super::account::status(context, name),
        InvocationKind::AccountRemove {
            name, consented, ..
        } => super::account::remove(context, &name, consented),
        InvocationKind::Completion { shell } => super::completion::run(context, shell),
        InvocationKind::Man => super::man::run(context),
        InvocationKind::Config { .. } => super::config::run(context),
        InvocationKind::Profile { .. } => super::profile::list(context),
        InvocationKind::VerbHelp { verb, subcommand } => {
            super::help::verb(context, verb, subcommand)
        }
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
