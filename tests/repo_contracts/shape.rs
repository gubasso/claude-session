//! The heading shapes markdownlint applies, checked against the constants that
//! own them.
//!
//! `MD043` takes one ordered heading array and markdownlint has no per-glob rule
//! configuration, so each fixed shape carries its array in its own file under
//! `.markdownlint/` and one `md-*` hook entry chooses the documents it applies
//! to. That array is a second statement of a shape this crate already fixes in a
//! constant, so this module asserts the two agree and keeps the constant the
//! owner.

use crate::violation::Violation;

/// Reads the `headings` array out of a shape config and compares it to the
/// constant that owns the shape. A missing file is a panic in `read`, so the
/// only findings here are a missing array and a disagreeing one.
pub(crate) fn matches(path: &str, text: &str, expected: &[&str]) -> Vec<Violation> {
    let Some(found) = headings(text) else {
        return vec![Violation::whole(path, "no MD043 headings array")];
    };
    if found == expected {
        return Vec::new();
    }
    vec![Violation::whole(
        path,
        format!("heading array disagrees with its constant: {found:?}"),
    )]
}

/// The project config is merged over the `--config` base the shape hooks pass,
/// so naming `MD043` there at any value — `false` included — switches off every
/// shape while the hooks keep reporting success. That failure is invisible in
/// hook output, which is exactly why it is checked here rather than left to a
/// comment.
pub(crate) fn absent_from_project_config(path: &str, text: &str) -> Vec<Violation> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            let code = line.find("//").map_or(*line, |at| &line[..at]);
            code.contains("\"MD043\"")
        })
        .map(|(index, _)| {
            Violation::at(
                path,
                index + 1,
                "MD043 must not appear in the project config; it overrides every shape",
            )
        })
        .collect()
}

/// A shape config only gates anything once a hook entry names it, so an array
/// no `--config` argument reads is decoration. This walks the shape directory
/// rather than a list of constants: a config added without its hook entry is
/// exactly the case a list of constants would not know about.
pub(crate) fn applied_by_a_hook(dir: &str, hook_text: &str) -> Vec<Violation> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let entries = std::fs::read_dir(&root)
        .unwrap_or_else(|err| panic!("{dir} is required by the repository gates: {err}"));

    let mut found: Vec<Violation> = Vec::new();
    let mut seen = 0_usize;
    for entry in entries {
        let name = entry.expect("a readable directory entry").file_name();
        let name = name.to_string_lossy().into_owned();
        if !name.ends_with(".markdownlint-cli2.jsonc") {
            continue;
        }
        seen += 1;
        let relative = format!("{dir}/{name}");
        if !hook_text.contains(&format!("'{relative}'")) {
            found.push(Violation::whole(
                &relative,
                "no hook entry names this shape config, so its array gates nothing",
            ));
        }
    }

    if seen == 0 {
        found.push(Violation::whole(dir, "no shape configs found"));
    }
    found
}

/// The file is JSONC, so line comments are dropped before the array is read.
/// This is a deliberate reader rather than a parser: it holds for the arrays of
/// string literals these files contain, and a `//` inside one of those strings
/// would defeat it.
fn headings(text: &str) -> Option<Vec<String>> {
    let bare: String = text
        .lines()
        .map(|line| line.find("//").map_or(line, |at| &line[..at]))
        .collect::<Vec<_>>()
        .join("\n");

    let after = &bare[bare.find("\"headings\"")? + "\"headings\"".len()..];
    let open = after.find('[')?;
    let close = after.find(']')?;
    if close < open {
        return None;
    }

    let mut items = Vec::new();
    let mut rest = &after[open + 1..close];
    while let Some(start) = rest.find('"') {
        let tail = &rest[start + 1..];
        let end = tail.find('"')?;
        items.push(tail[..end].to_string());
        rest = &tail[end + 1..];
    }
    Some(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::violation::assert_clean;

    const EXPECTED: &[&str] = &["*", "## Goal", "## Revisions"];
    const HEALTHY: &str = r###"
// A comment naming "headings" before the real one.
{
    "config": {
        "MD043": {
            "headings": [
                // The H1 varies.
                "*",
                "## Goal",
                "## Revisions"
            ]
        }
    }
}
"###;

    #[test]
    fn a_healthy_shape_config_is_accepted() {
        assert_clean(&matches("s.jsonc", HEALTHY, EXPECTED));
    }

    #[test]
    fn md043_in_the_project_config_is_a_violation() {
        let found = absent_from_project_config("p.jsonc", "{\n  \"MD043\": false\n}\n");
        assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
        assert!(found[0].to_string().contains("overrides every shape"));
    }

    /// The rule is about configuration, not about the prose explaining why the
    /// key is absent, so a commented mention stays legal.
    #[test]
    fn md043_named_only_in_a_comment_is_accepted() {
        assert_clean(&absent_from_project_config(
            "p.jsonc",
            "{\n  // \"MD043\" is deliberately absent.\n  \"MD046\": {}\n}\n",
        ));
    }

    /// Driven off the real shape directory, so the negative case is a hook file
    /// that has lost the entry naming one of the configs actually present.
    #[test]
    fn a_shape_config_no_hook_names_is_a_violation() {
        let hooks = crate::read(".pre-commit-config.yaml")
            .replace(".markdownlint/adr.markdownlint-cli2.jsonc", "gone.jsonc");
        let found = applied_by_a_hook(".markdownlint", &hooks);
        assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
        assert!(found[0].to_string().contains("gates nothing"));
    }

    #[test]
    fn an_empty_shape_directory_is_a_violation() {
        let found = applied_by_a_hook("docs/decisions", "");
        assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
        assert!(found[0].to_string().contains("no shape configs found"));
    }

    #[test]
    fn a_shape_config_without_an_array_is_a_violation() {
        let found = matches("s.jsonc", "{ \"config\": { \"MD046\": {} } }", EXPECTED);
        assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
        assert!(found[0].to_string().contains("no MD043 headings array"));
    }

    #[test]
    fn a_shape_config_that_disagrees_with_its_constant_is_a_violation() {
        let found = matches("s.jsonc", &HEALTHY.replace("## Goal", "## Aim"), EXPECTED);
        assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
        assert!(found[0].to_string().contains("disagrees with its constant"));
    }

    #[test]
    fn a_missing_heading_is_a_violation() {
        let found = matches("s.jsonc", &HEALTHY.replace("\"## Goal\",\n", ""), EXPECTED);
        assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    }

    #[test]
    fn a_reordered_array_is_a_violation() {
        let text = HEALTHY
            .replace("\"## Goal\",", "\"@@\",")
            .replace("\"## Revisions\"", "\"## Goal\"")
            .replace("\"@@\",", "\"## Revisions\",");
        let found = matches("s.jsonc", &text, EXPECTED);
        assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    }

    #[test]
    fn a_commented_out_heading_does_not_count() {
        let text = HEALTHY.replace("        \"## Revisions\"", "        // \"## Revisions\"");
        let found = matches("s.jsonc", &text, EXPECTED);
        assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    }
}
