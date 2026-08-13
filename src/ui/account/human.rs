//! The account reports a person reads.
//!
//! Every fact here is also in the `--json` document beside it, which is where a
//! script was always meant to look ([ADR-0093]). What this side drops is what a
//! person cannot use: a fingerprint, a second count, an exit code. What it adds
//! is the sentence that says why the fact matters and what to type next.
//!
//! [ADR-0093]: ../../../docs/decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md

#![allow(
    clippy::format_push_string,
    reason = "the renderer assembles one deterministic document before its sole write"
)]

use crate::{
    domain::{
        account::{
            AccountFinding, AccountStatus, AuthModeMetadata, ProfileBinding, Removal, ReportMode,
            SelectionSource, Warning,
        },
        checks::CheckStatus,
        config::Source,
        identifier::Identifier,
    },
    ui::prose::{BODY_INDENT, Palette, paragraph, plural, wrap},
};

use super::Binding;

/// Lays one row out plain, wraps it, then colours the status word.
fn row(palette: Palette, status: CheckStatus, text: &str) -> String {
    let token = format!("[{}]", status.as_str());
    let pad = " ".repeat(BODY_INDENT.saturating_sub(2 + token.chars().count()));
    let body = " ".repeat(BODY_INDENT);
    palette.recolour(wrap(text, &format!("  {token}{pad}"), &body), status)
}

/// The status word an account's local usability earns.
const fn usability(usable: bool) -> CheckStatus {
    if usable {
        CheckStatus::Pass
    } else {
        CheckStatus::Fail
    }
}

/// How this run came to be about this account, in words.
///
/// The environment rungs name their variable because that is a string the
/// reader has to go and change; the marker rung names no file, because there is
/// nothing there for them to edit.
const fn chose_account(source: SelectionSource) -> &'static str {
    match source {
        SelectionSource::Flag => "because you named it with --account",
        SelectionSource::Environment => {
            "because CLAUDE_SESSION_RS_DEFAULT_ACCOUNT names it in your environment"
        }
        SelectionSource::UserConfig => {
            "because default_account names it in your configuration file"
        }
        SelectionSource::Marker => "because it is the account the last launch used",
        SelectionSource::None => "",
    }
}

/// How a profile came to be the one in force, in words.
const fn chose_profile(source: Source) -> &'static str {
    match source {
        Source::Cli => "because you named it with --profile",
        Source::Environment => {
            "because CLAUDE_SESSION_RS_DEFAULT_PROFILE names it in your environment"
        }
        Source::Project => "because the project configuration file in this tree names it",
        Source::Account => "because it is the profile this account is bound to",
        Source::User => "because default_profile names it in your configuration file",
        Source::Default => "",
    }
}

/// How an account signs in, as a clause a sentence can be built around.
const fn signs_in(mode: ReportMode) -> &'static str {
    match mode {
        ReportMode::Login => "signs in with a saved login the child owns and refreshes",
        ReportMode::Token => "signs in with a long-lived token this wrapper stores",
        ReportMode::Invalid => {
            "has no readable record of how it signs in, so nothing here can use it"
        }
    }
}

/// Says how long ago something was, at the coarsest honest unit.
///
/// A person reading a token's age wants "three weeks ago", not a second count;
/// the second count is in `--json` for whoever needs to compare two of them.
fn ago(seconds: u64) -> String {
    let (count, unit) = match seconds {
        0..=89 => return "just now".to_owned(),
        90..=5399 => (seconds / 60, "minute"),
        5400..=86_399 => (seconds / 3600, "hour"),
        86_400..=2_591_999 => (seconds / 86_400, "day"),
        2_592_000..=31_535_999 => (seconds / 2_592_000, "month"),
        _ => (seconds / 31_536_000, "year"),
    };
    let count = count.max(1);
    let unit = if count == 1 {
        unit.to_owned()
    } else {
        format!("{unit}s")
    };
    format!("{count} {unit} ago")
}

/// Renders every account and which one this run would use.
pub(super) fn list(
    palette: Palette,
    source: SelectionSource,
    accounts: &[AccountFinding],
) -> String {
    if accounts.is_empty() {
        return format!(
            "{}\n\n{}",
            palette.heading("Accounts"),
            paragraph(concat!(
                "No accounts are stored on this machine yet. Every launch needs one, ",
                "so create the first with: claude-session-rs account login <name> ",
                "--profile <name>"
            ))
        );
    }
    let mut rows = String::new();
    for account in accounts {
        let mut text = format!("{} — {}", account.name.as_str(), signs_in(account.mode));
        match account.profile.as_ref() {
            // "is bound to" rather than "runs with": the listing reads local
            // state and knows nothing about which profile any given run would
            // resolve, and a flag, the environment, or a project file each
            // outranks the binding. `account status` is the verb that reports
            // the profile actually in force, and it is the one that says so.
            Some(profile) => {
                text.push_str(&format!(
                    ", and is bound to the \"{}\" profile",
                    profile.as_str()
                ));
            }
            None => text.push_str(&format!(
                ". It is bound to no profile, so a launch under it refuses. \
                Bind one with: claude-session-rs account bind {} --profile <name>",
                account.name.as_str()
            )),
        }
        if !account.usable {
            text.push_str(
                ". Its stored authentication is missing or unusable, so it cannot sign in",
            );
        }
        if !text.ends_with('.') {
            text.push('.');
        }
        rows.push_str(&row(palette, usability(account.usable), &text));
    }
    let selected = accounts.iter().find(|account| account.selected);
    let closing = selected.map_or_else(
        || {
            format!(
                "No account is selected, so a launch refuses until one is. Choose \
                one for a single run with --account <name>, or for every run with \
                default_account in your configuration file. There {} to choose from.",
                plural(accounts.len(), "is one", "are several"),
            )
        },
        |account| {
            format!(
                "This run would use {}, {}.",
                account.name.as_str(),
                chose_account(source)
            )
        },
    );
    format!(
        "{}\n\n{rows}\n{}",
        palette.heading("Accounts"),
        paragraph(&closing)
    )
}

/// Renders one account's health, its profile, and what is shadowing it.
pub(super) fn status(palette: Palette, status: &AccountStatus) -> String {
    let mut rows = String::new();
    rows.push_str(&row(
        palette,
        usability(status.usable),
        &authentication(status),
    ));
    rows.push_str(&row(palette, profile_status(status), &profile(status)));
    if let Some(probe) = status.child_probe {
        let (level, text) = match probe.status {
            crate::domain::account::ProbeStatus::Ok => (
                CheckStatus::Pass,
                "claude accepted this account's stored token.".to_owned(),
            ),
            crate::domain::account::ProbeStatus::Failed => (
                CheckStatus::Fail,
                "claude refused this account's stored token, so a launch under it \
                will fail. Replace it with: claude-session-rs account login "
                    .to_owned()
                    + status.account.as_str()
                    + " --token",
            ),
            crate::domain::account::ProbeStatus::Unavailable => (
                CheckStatus::Skipped,
                "claude could not be run, so the stored token was not checked \
                against it. Every other fact here is local and stands."
                    .to_owned(),
            ),
        };
        rows.push_str(&row(palette, level, &text));
    }
    for warning in &status.warnings {
        rows.push_str(&row(palette, CheckStatus::Warn, &warned(*warning)));
    }
    let closing = if status.selected {
        format!(
            "This is the account a launch would use, {}.",
            chose_account(status.selection_source.unwrap_or(SelectionSource::None))
        )
    } else {
        format!(
            "This is not the account a launch would use. Select it for one run \
            with: claude-session-rs --account {} ",
            status.account.as_str()
        )
    };
    format!(
        "{}\n\n{rows}\n{}",
        palette.heading(&format!("Account {}", status.account.as_str())),
        paragraph(&closing)
    )
}

/// The authentication sentence, with the token clocks folded in.
fn authentication(status: &AccountStatus) -> String {
    let mut text = format!("This account {}", signs_in(status.mode));
    if let Some(age) = status.age_seconds {
        text.push_str(&format!(", recorded {}", ago(age)));
    }
    if let Some(expiry) = status.estimated_expiry.as_ref() {
        text.push_str(&format!(
            ". It is estimated to stop working around {}, which is a guess from \
            when it was recorded rather than anything the token itself says",
            date(expiry)
        ));
    } else if status.metadata_consistent == Some(false) {
        text.push_str(
            ". Its stored record describes a different token than the one on disk, so \
            its age and expiry are withheld rather than guessed. The token itself was \
            proven to work, and the next login repairs the pair",
        );
    }
    if !status.usable && status.mode != ReportMode::Invalid {
        text.push_str(". Its stored credential is missing or unusable, so a launch under it fails");
    }
    text.push('.');
    text
}

/// The profile row's status: an unbound or dangling binding is a warning,
/// because a launch under it refuses and nothing else here says so.
const fn profile_status(status: &AccountStatus) -> CheckStatus {
    match (status.profile.is_some(), status.profile_present) {
        (true, Some(true) | None) => CheckStatus::Pass,
        _ => CheckStatus::Warn,
    }
}

fn profile(status: &AccountStatus) -> String {
    let Some(profile) = status.profile.as_ref() else {
        return format!(
            "It is bound to no profile, so a launch under it refuses before claude \
            starts. Bind one with: claude-session-rs account bind {} --profile <name>",
            status.account.as_str()
        );
    };
    if status.profile_present == Some(false) {
        return format!(
            "It is bound to the \"{}\" profile, which has no document, so a launch \
            under it refuses. Write that profile, or bind another with: \
            claude-session-rs account bind {} --profile <name>",
            profile.as_str(),
            status.account.as_str()
        );
    }
    // The binding is what the account carries; the layer that wins is what this
    // run would actually use. Saying only that a different one would be used
    // left the reader without the one fact they need to check it, so the
    // overriding profile is named whenever it is known and differs.
    let mut text = format!("It is bound to the \"{}\" profile", profile.as_str());
    match (status.profile_source, status.effective_profile.as_ref()) {
        (Some(source), Some(effective))
            if source != Source::Account && source != Source::Default && effective != profile =>
        {
            text.push_str(&format!(
                ", but this run would use the \"{}\" profile instead, {}",
                effective.as_str(),
                chose_profile(source)
            ));
        }
        _ => text.push_str(", and that is what this run would use"),
    }
    text.push('.');
    text
}

/// The one wording each warning carries, said as a consequence.
fn warned(warning: Warning) -> String {
    let mut text = warning.message();
    if let Some(first) = text.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    text.push('.');
    text
}

/// Renders a completed login.
pub(super) fn login(
    palette: Palette,
    account: &str,
    path: &std::path::Path,
    metadata: &AuthModeMetadata,
    binding: &Binding,
) -> String {
    let mut rows = row(
        palette,
        CheckStatus::Pass,
        &format!(
            "It {}, recorded just now.",
            signs_in(metadata.mode.report())
        ),
    );
    let profile_row = if binding.present {
        format!(
            "It runs with the \"{}\" profile, {}.",
            binding.profile.as_str(),
            chose_profile(binding.source)
        )
    } else {
        format!(
            "It is bound to the \"{}\" profile, {}, but that profile has no document \
            yet. Write it before the next launch, or bind another with: \
            claude-session-rs account bind {account} --profile <name>",
            binding.profile.as_str(),
            chose_profile(binding.source)
        )
    };
    rows.push_str(&row(
        palette,
        if binding.present {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        &profile_row,
    ));
    // Always a pass, because the login refused rather than reported if this had
    // not been recorded. It is here at all because it is the half of "ready" a
    // reader cannot check: the credential half is what the two rows above say,
    // and this is what a first launch used to fail on silently.
    //
    // The claim narrows when the profile has no document, because the row above
    // has just said that a launch refuses before the child starts. Stating both
    // would make the report contradict itself, so the recorded fact is reported
    // without the whole-launch promise it does not on its own establish.
    rows.push_str(&row(
        palette,
        CheckStatus::Pass,
        if binding.present {
            "A launch under it goes straight to claude's prompt, with no first-run setup."
        } else {
            "Claude's first-run setup is recorded as done, so it will not stand between \
            this account and the prompt."
        },
    ));
    format!(
        "{}\n\n{rows}\n{}",
        palette.heading(&format!("Account {account} is ready")),
        paragraph(&format!("Its files are at {}.", path.display()))
    )
}

/// Renders a new binding.
pub(super) fn binding(palette: Palette, account: &Identifier, binding: &ProfileBinding) -> String {
    format!(
        "{}\n\n{}",
        palette.heading(&format!(
            "Account {} now runs with the \"{}\" profile",
            account.as_str(),
            binding.profile.as_str()
        )),
        paragraph("This takes effect on the next launch under this account.")
    )
}

/// Renders what one removal did, or that nothing was removed.
pub(super) fn removal(palette: Palette, removal: &Removal) -> String {
    if !removal.removed {
        return format!(
            "{}\n\n{}",
            palette.heading(&format!(
                "Account {} is untouched",
                removal.account.as_str()
            )),
            paragraph("Nothing was removed, because the removal was not confirmed.")
        );
    }
    format!(
        "{}\n\n{}",
        palette.heading(&format!("Account {} is removed", removal.account.as_str())),
        paragraph(&format!(
            "Everything it stored under {} is gone{}.",
            removal.path.display(),
            if removal.marker_cleared == Some(true) {
                ", and it is no longer the account a bare launch would use"
            } else {
                ""
            }
        ))
    )
}

/// Trims a timestamp to the day, which is the only part an estimate supports.
fn date(timestamp: &str) -> &str {
    timestamp.get(0..10).unwrap_or(timestamp)
}
