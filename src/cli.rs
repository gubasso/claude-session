//! Clap parse-shape only; opaque child arguments never enter this parser.

use std::path::PathBuf;

use crate::domain::identifier::Identifier;
use clap::{ArgAction, Parser, Subcommand};

pub(crate) mod account;
pub(crate) mod doctor;
pub(crate) mod help;
pub(crate) mod version;

/// The small wrapper-owned grammar.
#[derive(Debug, Parser)]
#[command(
    name = "claude-session",
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
    /// Inspect wrapper and child health.
    Doctor(doctor::DoctorArgs),
    /// Show wrapper help followed by native help.
    Help(help::HelpArgs),
    /// Show wrapper and native version information.
    Version(version::VersionArgs),
}
