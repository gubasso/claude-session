//! Clap parse-shape only; opaque child arguments never enter this parser.

use std::path::PathBuf;

use crate::domain::identifier::Identifier;
use clap::{ArgAction, Parser, Subcommand};

pub(crate) mod account;
pub(crate) mod completion;
pub(crate) mod doctor;
pub(crate) mod help;
pub(crate) mod man;
pub(crate) mod profile;
pub(crate) mod version;

/// The small wrapper-owned grammar.
// `about` and `version` are set explicitly because the man page publishes both
// and `--help` publishes neither: `long_about` shadows `about` in help, and the
// automatic version flag is disabled. Left to the derive, the page's `NAME`
// section — what `whatis` and `apropos` index — would carry this struct's own
// doc comment, and its `.TH` title would carry an empty version. Setting
// `version` adds metadata only; `disable_version_flag` still owns the spelling,
// which `--version` composes with the child's (`cli-surface.md#version-output`).
#[derive(Debug, Parser)]
#[command(
    name = "claude-session",
    about = "preserve native claude behavior while adding isolated sessions",
    version = env!("CARGO_PKG_VERSION"),
    disable_version_flag = true,
    disable_help_flag = true,
    infer_long_args = false,
    disable_help_subcommand = true,
    long_about = include_str!("ui/help.txt")
)]
pub(crate) struct Cli {
    /// Increase terminal diagnostics, repeated up to trace.
    #[arg(long, action = ArgAction::Count, global = true)]
    pub(crate) verbose: u8,
    /// Suppress non-error wrapper diagnostics.
    #[arg(long, short = 'q', global = true, conflicts_with = "verbose")]
    pub(crate) quiet: bool,
    /// Replace the user configuration file.
    #[arg(long, global = true, value_name = "PATH")]
    pub(crate) config: Option<PathBuf>,
    /// Select an account for later composition slices.
    #[arg(long, global = true)]
    pub(crate) account: Option<Identifier>,
    /// Select a profile for later composition slices.
    #[arg(long, global = true)]
    pub(crate) profile: Option<Identifier>,
    /// Request the composed version surface.
    #[arg(long = "version", short = 'V', action = ArgAction::SetTrue)]
    pub(crate) version_flag: bool,
    /// Request the composed help surface.
    #[arg(long = "help", short = 'h', action = ArgAction::SetTrue)]
    pub(crate) help_flag: bool,
    /// Wrapper-owned read-only commands.
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

/// Implemented wrapper verbs only.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Manage durable child-owned accounts.
    Account(account::AccountArgs),
    /// Emit shell completions for the wrapper's grammar.
    Completion(completion::CompletionArgs),
    /// Inspect wrapper and child health.
    Doctor(doctor::DoctorArgs),
    /// Show wrapper help followed by native help.
    Help(help::HelpArgs),
    /// Emit a man page generated from the wrapper's grammar.
    Man(man::ManArgs),
    /// List the profiles available to select.
    Profile(profile::ProfileArgs),
    /// Show wrapper and native version information.
    Version(version::VersionArgs),
}
