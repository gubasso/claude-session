//! Renderers for the generated configuration artifacts.
//!
//! In the shipped crate rather than in `xtask` for one reason: package
//! integration tests link the library target, so `tests/` can call these
//! directly and prove freshness without shelling out to cargo. `xtask` is a thin
//! CLI over them, and nothing under `src` may reference it
//! ([ADR-0014](../../docs/decisions/ADR-0014-xtask-workspace-for-dev-tooling.md)).
//!
//! Every artifact is LF-normalized and ends with exactly one newline. Rendering
//! is deterministic, so identical types produce identical bytes and freshness is
//! a byte comparison rather than a cache.

#![allow(
    unreachable_pub,
    reason = "these items are reachable through the library target"
)]
#![allow(
    clippy::format_push_string,
    reason = "each renderer assembles one deterministic artifact before returning it"
)]

use super::schema::{KEYS, PROFILE_FIELDS};

/// The lines every generated artifact opens with, before its own body.
///
/// A header names the copy destination and states that the wrapper never writes
/// configuration, because a reader who found this file in a repository has to
/// know it is a template rather than the live file
/// (`configuration.md#what-a-generated-example-contains`).
fn header(kind: &str, source: &str, destination: &[&str]) -> String {
    let mut out = String::new();
    let mut line = |text: &str| {
        out.push_str(text);
        out.push('\n');
    };
    line(&format!("# Example {kind} for claude-session."));
    line("#");
    line(&format!(
        "# GENERATED from {source}. Do not edit in place; edit the"
    ));
    line("# type and regenerate with `cargo xtask gen-config`.");
    line("#");
    line("# Copy it where it belongs and edit it there:");
    for step in destination {
        line(&format!("#     {step}"));
    }
    line("#");
    line("# The wrapper never writes your configuration. A missing file is not an error.");
    line("#");
    line("# Every value below is an obviously fake placeholder. Replace before use.");
    out
}

/// Renders the copyable configuration example.
///
/// Every key is optional, so the uncommented body is already a minimal valid
/// configuration and each commented key is independently usable.
#[must_use]
pub fn config_example_toml() -> String {
    let mut out = header(
        "configuration",
        "the wrapper's configuration type",
        &[
            "cp docs/reference/examples/config.example.toml \\",
            "   \"${XDG_CONFIG_HOME:-$HOME/.config}/claude-session/config.toml\"",
        ],
    );
    for key in KEYS {
        out.push('\n');
        for line in wrapped(key.description) {
            out.push_str(&format!("# {line}\n"));
        }
        // Optional keys are commented out; a required one would be active. Every
        // key is optional today, which is why none of them is.
        let prefix = if key.required { "" } else { "# " };
        // A boolean placeholder is a TOML literal, not a string: quoting it
        // would produce an example the loader refuses for the one reason an
        // example must never fail, its own type.
        let value = if key.type_name == "boolean" {
            key.placeholder.to_owned()
        } else {
            format!("\"{}\"", key.placeholder)
        };
        out.push_str(&format!("{prefix}{} = {value}\n", key.name));
    }
    out
}

/// Renders the JSON Schema for the configuration file.
///
/// `additionalProperties` is `false`, matching the `deny_unknown_fields` the
/// loader decodes with, so the schema and the program agree about what a valid
/// file is.
#[must_use]
pub fn config_schema_json() -> String {
    let mut properties = String::new();
    for (index, key) in KEYS.iter().enumerate() {
        if index > 0 {
            properties.push_str(",\n");
        }
        // One member per line, always. The alternative is matching the JSON
        // formatter's width rule, which would silently stop matching the first
        // time a description grew — and a generator that fights the formatter
        // can never be green (`configuration.md#freshness`).
        properties.push_str(&format!(
            "    \"{}\": {{\n      \"type\": \"{}\",\n      \"description\": \"{}\"\n    }}",
            key.name,
            key.type_name,
            escape(key.description)
        ));
    }
    let required: Vec<&str> = KEYS
        .iter()
        .filter(|key| key.required)
        .map(|key| key.name)
        .collect();
    let required_line = if required.is_empty() {
        String::new()
    } else {
        format!(
            "  \"required\": [{}],\n",
            required
                .iter()
                .map(|name| format!("\"{name}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\n");
    out.push_str("  \"title\": \"claude-session configuration\",\n");
    out.push_str("  \"type\": \"object\",\n");
    out.push_str("  \"additionalProperties\": false,\n");
    out.push_str(&required_line);
    out.push_str("  \"properties\": {\n");
    out.push_str(&properties);
    out.push_str("\n  }\n");
    out.push_str("}\n");
    out
}

/// Renders the profile example from the profile type's field descriptor.
#[must_use]
pub fn profile_example_yaml() -> String {
    let mut out = header(
        "profile",
        "the profile type",
        &[
            "cp docs/reference/examples/profile.example.yaml \\",
            "   \"${XDG_CONFIG_HOME:-$HOME/.config}/claude-session/profiles/dev.yaml\"",
        ],
    );
    for field in PROFILE_FIELDS {
        out.push('\n');
        for line in wrapped(field.description) {
            out.push_str(&format!("# {line}\n"));
        }
        out.push_str(&format!("{}:\n", field.name));
        for line in field.body {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Escapes a description for a JSON string literal.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Wraps a description onto comment lines at a fixed width.
///
/// Fixed rather than terminal-derived: these are files, and a width that
/// depended on the generating terminal would make the output non-deterministic.
fn wrapped(text: &str) -> Vec<String> {
    const WIDTH: usize = 74;
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.len() + 1 + word.len() > WIDTH {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Returns every generated artifact as `(repository-relative path, contents)`.
///
/// One list, so the generator, the freshness check, and the tests cannot
/// disagree about which files are generated.
#[must_use]
pub fn artifacts() -> Vec<(&'static str, String)> {
    vec![
        (
            "docs/reference/examples/config.example.toml",
            config_example_toml(),
        ),
        (
            "docs/reference/examples/config.schema.json",
            config_schema_json(),
        ),
        (
            "docs/reference/examples/profile.example.yaml",
            profile_example_yaml(),
        ),
    ]
}
