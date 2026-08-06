//! Compares the wrapper's claimed surface with the measured child inventory.
//! Owns no input and no assertions: it returns violations and the caller
//! decides, which is what lets the negative tests drive it from literals.

use std::fmt::{self, Display, Formatter};

use crate::inventory::Inventory;
use crate::surface::{Resolution, Status, Surface};

pub(crate) const SURFACE: &str = "docs/reference/cli-surface.md";
pub(crate) const FIXTURE: &str = "tests/fixtures/child-inventory.yaml";

#[derive(Debug)]
pub(crate) struct Violation {
    file: &'static str,
    line: Option<usize>,
    message: String,
}

impl Violation {
    pub(crate) fn at(file: &'static str, line: usize, message: impl Into<String>) -> Self {
        Self {
            file,
            line: Some(line),
            message: message.into(),
        }
    }

    /// A fact about the whole file, reported without a line, because a
    /// cross-file finding has no single line to blame.
    pub(crate) fn whole(file: &'static str, message: impl Into<String>) -> Self {
        Self {
            file,
            line: None,
            message: message.into(),
        }
    }
}

impl Display for Violation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "{}:{}: {}", self.file, line, self.message),
            None => write!(f, "{}: {}", self.file, self.message),
        }
    }
}

/// Sorted so a failure reads the same on every host and every run.
pub(crate) fn render(violations: &[Violation]) -> String {
    let mut lines: Vec<String> = violations.iter().map(ToString::to_string).collect();
    lines.sort();
    let count = violations.len();
    lines.push(format!(
        "{count} violation{}",
        if count == 1 { "" } else { "s" }
    ));
    format!("\n{}", lines.join("\n"))
}

pub(crate) fn flags(surface: &Surface, inventory: &Inventory) -> Vec<Violation> {
    let mut out = Vec::new();
    for claimed in &surface.flags {
        let spelling = &claimed.spelling;
        if let Some(meaning) = inventory.aliases.get(spelling) {
            out.push(Violation::at(
                SURFACE,
                claimed.line,
                format!(
                    "`{spelling}` is the child's spelling for `{meaning}`, so \
                    claiming it would change a token's meaning rather than shadow \
                    it; drop the claim (ADR-0044)"
                ),
            ));
            continue;
        }
        let in_child = inventory.flags.contains(spelling);
        match (&claimed.status, in_child) {
            (Status::Free, true) => out.push(Violation::at(
                SURFACE,
                claimed.line,
                format!(
                    "`{spelling}` is in the {} child inventory and the child status \
                    column says free; name the child's meaning in that column, or \
                    drop the claim",
                    inventory.child_version
                ),
            )),
            (Status::Collides, false) => out.push(Violation::at(
                SURFACE,
                claimed.line,
                format!(
                    "the child status column claims `{spelling}` collides and the {} \
                    inventory does not list it; re-measure the fixture, or drop the \
                    collision note",
                    inventory.child_version
                ),
            )),
            _ => {}
        }
    }
    out
}

pub(crate) fn verbs(surface: &Surface, inventory: &Inventory) -> Vec<Violation> {
    let mut out = Vec::new();

    for claimed in &surface.verbs {
        let owned = inventory.verbs.contains(&claimed.name);
        let resolved = surface.overlaps.iter().any(|o| o.verb == claimed.name);
        if owned && !resolved {
            out.push(Violation::at(
                SURFACE,
                claimed.line,
                format!(
                    "the child owns verb `{}` and the verb-overlap table does not \
                    name it; add a row resolving it as renamed or composed",
                    claimed.name
                ),
            ));
        }
    }

    for overlap in &surface.overlaps {
        let verb = &overlap.verb;
        if !inventory.verbs.contains(verb) {
            out.push(Violation::at(
                SURFACE,
                overlap.line,
                format!(
                    "the overlap table resolves `{verb}` and the {} inventory does \
                    not list it as a child verb; re-measure the fixture, or drop \
                    the row",
                    inventory.child_version
                ),
            ));
        }
        let claimed = surface.verbs.iter().any(|v| &v.name == verb);
        match overlap.resolution {
            // The two rows below are the `auth` and `doctor` asymmetry
            // mechanized: a renamed verb must have left the claimed table, and a
            // composed one must still be in it. Neither name is hardcoded.
            Resolution::Renamed if claimed => out.push(Violation::at(
                SURFACE,
                overlap.line,
                format!(
                    "the overlap table resolves `{verb}` as renamed and the \
                    wrapper-verb table still claims it; drop the claimed row, or \
                    resolve the overlap as composed"
                ),
            )),
            Resolution::Composed if !claimed => out.push(Violation::at(
                SURFACE,
                overlap.line,
                format!(
                    "the overlap table resolves `{verb}` as composed and the \
                    wrapper-verb table does not claim it; there is nothing to \
                    compose the child's output with"
                ),
            )),
            _ => {}
        }
        if overlap.resolution == Resolution::Composed
            && !inventory.subcommand_flags.contains_key(verb)
        {
            out.push(Violation::whole(
                FIXTURE,
                format!(
                    "subcommand_flags has no `{verb}` key and the surface composes \
                    that verb; measure the child's own flags for it"
                ),
            ));
        }
    }

    out
}

/// The prose in the surface names the same version and date the fixture carries.
/// Without this, regenerating the fixture and forgetting the prose leaves the
/// page a confident lie that nothing else would catch.
pub(crate) fn datelines(surface_text: &str, inventory: &Inventory) -> Vec<Violation> {
    let mut out = Vec::new();
    if !surface_text.contains(&inventory.child_version) {
        out.push(Violation::whole(
            SURFACE,
            format!(
                "the fixture was measured against child version {} and no prose \
                here names that version",
                inventory.child_version
            ),
        ));
    }
    if !surface_text.contains(&inventory.measured_on) {
        out.push(Violation::whole(
            SURFACE,
            format!(
                "the fixture was measured on {} and no prose here names that date",
                inventory.measured_on
            ),
        ));
    }
    out
}
