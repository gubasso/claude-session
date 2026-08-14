//! Supplying the user's own child assets to every session directory.
//!
//! This module is for linking one machine-local tree into the directory a
//! launch just created. It models nothing any asset contains: the wrapper
//! supplies a directory and the child reads it, which is the whole carry
//! ([ADR-0088]).
//!
//! [ADR-0088]: ../../docs/decisions/ADR-0088-model-nothing-the-child-already-owns.md
//! [ADR-0106]: ../../docs/decisions/ADR-0106-supply-child-assets-from-one-tree.md

use std::path::{Path, PathBuf};

use crate::{
    adapters::filesystem::SystemFileSystem, context::AppContext, error::AppError,
    services::storage::guard,
};

/// The child's published user-scope asset names.
///
/// Membership is a fact about the child rather than a wrapper preference, which
/// is why the list is its published tree and not a shorter one somebody found
/// convenient. Each name is a child-owned fact carried against the launch
/// obligation of [ADR-0089]; none is reachable by the discovery scan, because
/// every one of them is an ordinary English word that would match this
/// sentence.
///
/// `plugins` is deliberately absent ([ADR-0106]).
///
/// [ADR-0089]: ../../docs/decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md
const ASSETS: &[&str] = &[
    "skills",
    "agents",
    "commands",
    "rules",
    "workflows",
    "output-styles",
    "themes",
    "agent-memory",
    "CLAUDE.md",
    "keybindings.json",
];

/// Links every asset the user's tree holds into one session directory.
///
/// An absent member is skipped rather than materialised empty: an empty
/// directory asserts the user has none of that asset, and not having written
/// any yet is the ordinary case.
///
/// The asset tree itself is the user's, outside the wrapper's managed region,
/// so it is read for presence and never validated, corrected, or created.
pub(crate) fn supply(context: &AppContext, session: &Path) -> Result<(), AppError> {
    let tree = context.paths().assets();
    for name in ASSETS {
        let source = tree.join(name);
        // An asset that cannot be inspected is not an absent one. Treating the
        // two alike would drop a skill the user wrote because its tree was
        // unreadable, and say nothing about why.
        let seat = session.join(name);
        match SystemFileSystem::look(&source) {
            // An absent asset supplies nothing, but the name it would have
            // taken is still inspected. Skipping the inspection would let an
            // occupant the wrapper never declared sit at a name the child
            // reads, for as long as the tree happens to lack that asset.
            Ok(None) => {
                guard::validate(
                    context.paths().state(),
                    &seat,
                    guard::Expected::DeclaredLink(&source),
                )?;
                continue;
            }
            Ok(Some(_)) => {}
            // Warned rather than refused, and the warning is the whole point:
            // an asset is a convenience, and refusing to start claude because
            // one directory in the user's own tree could not be stat'ed is out
            // of proportion to what was lost. Warning also keeps this agreeing
            // with the soft `session-assets-linked` row, which ADR-0018
            // requires: a guard that refuses where doctor reports a warning
            // makes the report a lie.
            Err(error) => {
                tracing::warn!(
                    op = "supply_assets",
                    status = "skipped",
                    path = %source.display(),
                    "{} could not be inspected, so this session starts without it: {}",
                    source.display(),
                    error
                );
                continue;
            }
        }
        guard::ensure_link(context.paths().state(), &seat, &source)?;
    }
    Ok(())
}

/// What the catalog check found when it looked at the asset tree.
///
/// An asset that cannot be inspected is its own answer, not an absent one:
/// reporting a tree the wrapper could not read as a tree holding nothing gives
/// the reader a population step to perform when the problem is a permission.
pub(crate) enum Survey {
    /// The names the tree holds, which may be none of them.
    Held(Vec<&'static str>),
    /// The first asset that could not be inspected, and why.
    Uninspectable(PathBuf, String),
}

/// Reports which assets a launch would supply, for the catalog check.
pub(crate) fn present(context: &AppContext) -> Survey {
    let tree = context.paths().assets();
    let mut held = Vec::new();
    for name in ASSETS {
        let source = tree.join(name);
        match SystemFileSystem::look(&source) {
            Ok(Some(_)) => held.push(*name),
            Ok(None) => {}
            Err(error) => return Survey::Uninspectable(source, error.to_string()),
        }
    }
    Survey::Held(held)
}

/// Returns every name a launch may supply, whether or not the tree holds it.
pub(crate) const fn declared() -> &'static [&'static str] {
    ASSETS
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The set is the child's published user-scope tree. A name added without a
    /// reason is the thing this pins against.
    #[test]
    fn the_declared_set_is_the_published_tree() {
        assert_eq!(
            declared(),
            &[
                "skills",
                "agents",
                "commands",
                "rules",
                "workflows",
                "output-styles",
                "themes",
                "agent-memory",
                "CLAUDE.md",
                "keybindings.json",
            ]
        );
    }

    /// The child validates a marketplace's install location by string prefix,
    /// so a linked plugins directory reports as corrupted.
    #[test]
    fn the_plugin_tree_is_never_supplied() {
        assert!(!declared().contains(&"plugins"));
    }
}
