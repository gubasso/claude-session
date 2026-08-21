//! The configuration report a person reads.
//!
//! Same source and same field set as the machine document beside it. What
//! changes is that a layer is named in words rather than as a bracketed token,
//! and a defect leads with what it costs rather than with its id ([ADR-0093]).
//!
//! [ADR-0093]: ../../../docs/decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md

use crate::{
    commands::config::{ProfileReport, Report},
    domain::{checks::CheckStatus, config::Source},
    ui::prose::{BODY_INDENT, Palette, paragraph, plural, wrap},
};

/// Names the layer that supplied a value, as the end of a sentence.
const fn layer(source: Source) -> &'static str {
    match source {
        Source::Cli => "from the command line",
        Source::Environment => "from the environment",
        Source::Project => "from the project configuration file in this tree",
        Source::Account => "from the selected account's own profile binding",
        Source::User => "from your configuration file",
        Source::Default => "from nowhere; no layer set it",
    }
}

fn row(palette: Palette, status: CheckStatus, text: &str) -> String {
    let token = format!("[{}]", status.as_str());
    let pad = " ".repeat(BODY_INDENT.saturating_sub(2 + token.chars().count()));
    let body = " ".repeat(BODY_INDENT);
    palette.recolour(wrap(text, &format!("  {token}{pad}"), &body), status)
}

/// Renders the whole report.
pub(super) fn report(palette: Palette, report: &Report) -> String {
    let mut out = String::new();
    out.push_str(&palette.heading("Settings"));
    out.push_str("\n\n");
    for key in &report.configuration {
        out.push_str(&paragraph(&key.value.as_ref().map_or_else(
            || format!("{} is not set by any layer.", key.name),
            |value| format!("{} is {value}, {}.", key.name, layer(key.source)),
        )));
    }

    out.push('\n');
    out.push_str(&palette.heading("Files consulted"));
    out.push_str("\n\n");
    if report.files.is_empty() {
        out.push_str(&paragraph(
            "No configuration file was looked for, so every value above came from \
            a flag, the environment, or a built-in default.",
        ));
    } else {
        for file in &report.files {
            let opening = match file.source {
                Source::Project => "The project configuration file for this tree",
                Source::User => "Your configuration file",
                other => layer(other),
            };
            out.push_str(&paragraph(&format!(
                "{opening} is {}, and {}.",
                file.path.display(),
                if file.existed {
                    "it was read"
                } else {
                    "it is not there, so nothing was read from it"
                }
            )));
        }
    }

    out.push('\n');
    out.push_str(&palette.heading("Profile"));
    out.push_str("\n\n");
    out.push_str(&profile(report.profile.as_ref()));

    out.push('\n');
    out.push_str(&palette.heading("Problems"));
    out.push_str("\n\n");
    out.push_str(&defects(palette, report));

    out.push('\n');
    out.push_str(&paragraph(
        "The composed settings file is one native settings layer among several. \
        It is not claude's whole effective configuration.",
    ));
    out
}

/// Renders the profile section, or says why there is none.
fn profile(profile: Option<&ProfileReport>) -> String {
    let Some(profile) = profile else {
        return paragraph(
            "No profile is resolved, so a launch refuses before claude starts. Bind \
            one to the account with: claude-session account bind <account> \
            --profile <name>",
        );
    };
    let mut out = paragraph(&format!(
        "\"{}\" is the profile in force, written at {}.",
        profile.name.as_str(),
        profile.path.display()
    ));
    out.push_str(&paragraph(&match profile.pieces.len() {
        0 => "It names no settings pieces.".to_owned(),
        count => format!(
            "It composes {} settings {}, in the order it lists them: {}.",
            plural(count, "one", "these"),
            plural(count, "piece", "pieces"),
            profile
                .pieces
                .iter()
                .map(|piece| piece.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }));
    if !profile.strategies.is_empty() {
        out.push_str(&paragraph(&format!(
            "It declares {} array {}, so those lists are combined rather than \
            replaced: {}.",
            plural(profile.strategies.len(), "one", "these"),
            plural(profile.strategies.len(), "rule", "rules"),
            profile
                .strategies
                .iter()
                .map(|strategy| format!("{} by {}", strategy.pointer, strategy.strategy))
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    out.push_str(&paragraph(&if profile.exists {
        format!(
            "The composed settings claude will read are at {}.",
            profile.settings.display()
        )
    } else {
        format!(
            "Nothing is composed yet. The next launch writes {}.",
            profile.settings.display()
        )
    }));
    out
}

/// Renders the config-scoped catalog results, or says there are none.
fn defects(palette: Palette, report: &Report) -> String {
    let problems: Vec<&crate::domain::checks::CheckResult> = report
        .defects
        .iter()
        .filter(|defect| defect.status != CheckStatus::Pass)
        .collect();
    if problems.is_empty() {
        return paragraph("Nothing is wrong with the configuration this run resolved.");
    }
    let mut out = String::new();
    for defect in problems {
        let mut text = defect
            .reason
            .as_deref()
            .unwrap_or(defect.message.as_str())
            .to_owned();
        if let Some(hint) = defect.hint.as_ref() {
            text.push(' ');
            text.push_str(hint);
        }
        out.push_str(&row(palette, defect.status, &text));
    }
    out
}
