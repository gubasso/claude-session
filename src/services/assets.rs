//! Supplying the user's own child assets to every session directory.
//!
//! This module is for linking the directories a user maintains into the
//! directory a launch just created. It models nothing any asset contains: the
//! wrapper supplies a directory and the child reads it, which is the whole
//! carry ([ADR-0088]).
//!
//! Two sources, and the split is the point. Nine assets come from the
//! wrapper's own tree, which the user maintains by hand ([ADR-0106]). Skills
//! come from the directory the child itself publishes, because skills are the
//! one asset with installers, and an installer writes that path ([ADR-0125]).
//! [`declared`] is the one place that pairing is stated, so a launch, a
//! catalog row, and the declared-links check cannot disagree about it.
//!
//! [ADR-0088]: ../../docs/decisions/ADR-0088-model-nothing-the-child-already-owns.md
//! [ADR-0106]: ../../docs/decisions/ADR-0106-supply-child-assets-from-one-tree.md
//! [ADR-0125]: ../../docs/decisions/ADR-0125-supply-skills-from-the-native-directory.md

use std::path::{Path, PathBuf};

use crate::{
    adapters::filesystem::SystemFileSystem, context::AppContext, error::AppError,
    services::storage::guard,
};

/// The asset name supplied from the child's own configuration directory.
const NATIVE: &str = "skills";

/// The child's published user-scope asset names held in the wrapper's tree.
///
/// Membership is a fact about the child rather than a wrapper preference, which
/// is why the list is its published tree and not a shorter one somebody found
/// convenient. Each name is a child-owned fact carried against the launch
/// obligation of [ADR-0089]; none is reachable by the discovery scan, because
/// every one of them is an ordinary English word that would match this
/// sentence.
///
/// `plugins` is deliberately absent ([ADR-0106]). [`NATIVE`] is absent for a
/// different reason: it is supplied, from somewhere else ([ADR-0125]).
///
/// [ADR-0089]: ../../docs/decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md
const TREE_ASSETS: &[&str] = &[
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

/// Which of the two directories an asset comes from.
///
/// Carried on the declaration rather than recomputed from the name, so a
/// caller grouping by source reads the same answer the launch linked by. A
/// name comparison would be a second statement of the pairing, and the point
/// of [`declared`] is that there is only one.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    /// The wrapper's own tree, which the user maintains by hand.
    Tree,
    /// The child's own configuration directory, which installers write.
    Child,
}

/// One asset name and the directory a launch would link it from.
pub(crate) struct Declared {
    /// The name the child reads it under, which is also the seat's name.
    pub(crate) name: &'static str,
    /// The directory outside the wrapper's managed region that holds it.
    pub(crate) path: PathBuf,
    /// Which of the two directories that is.
    pub(crate) source: Source,
}

/// Returns every name a launch may supply, with the directory it comes from.
///
/// Every declared name, whether or not its source currently holds it: a launch
/// validates the seat either way, so a caller filtering by presence would leave
/// exactly the seat an emptied source makes the next launch refuse unchecked.
///
/// The list is the same length on every run. `$HOME` is required to resolve at
/// all, so the skill entry can never be the one that goes missing and leaves a
/// seat neither linked nor inspected.
pub(crate) fn declared(context: &AppContext) -> Vec<Declared> {
    let tree = context.paths().assets();
    let mut all = vec![Declared {
        name: NATIVE,
        path: context.paths().child_skills(),
        source: Source::Child,
    }];
    all.extend(TREE_ASSETS.iter().map(|name| Declared {
        name,
        path: tree.join(name),
        source: Source::Tree,
    }));
    all
}

/// Links every asset its source holds into one session directory.
///
/// An absent asset is skipped rather than materialised empty: an empty
/// directory asserts the user has none of that asset, and not having written
/// any yet is the ordinary case.
///
/// Every source is the user's, outside the wrapper's managed region, so each is
/// read for presence and never validated, corrected, or created.
pub(crate) fn supply(context: &AppContext, session: &Path) -> Result<(), AppError> {
    for asset in declared(context) {
        // An asset that cannot be inspected is not an absent one. Treating the
        // two alike would drop a skill the user wrote because its directory was
        // unreadable, and say nothing about why.
        let seat = session.join(asset.name);
        match SystemFileSystem::look(&asset.path) {
            // An absent asset supplies nothing, but the name it would have
            // taken is still inspected. Skipping the inspection would let an
            // occupant the wrapper never declared sit at a name the child
            // reads, for as long as the source happens to lack that asset.
            Ok(None) => {
                guard::validate(
                    context.paths().state(),
                    &seat,
                    guard::Expected::DeclaredLink(&asset.path),
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
                    path = %asset.path.display(),
                    "{} could not be inspected, so this session starts without it: {}",
                    asset.path.display(),
                    error
                );
                continue;
            }
        }
        guard::ensure_link(context.paths().state(), &seat, &asset.path)?;
    }
    Ok(())
}

/// What the catalog check found when it looked at the two sources.
///
/// An asset that cannot be inspected is its own answer, not an absent one:
/// reporting a directory the wrapper could not read as one holding nothing
/// gives the reader a population step to perform when the problem is a
/// permission.
pub(crate) enum Survey {
    /// What each source holds, either of which may be nothing.
    Held {
        /// The names the wrapper's own tree holds.
        tree: Vec<&'static str>,
        /// The child's own skill directory, when it exists.
        skills: Option<PathBuf>,
    },
    /// The first asset that could not be inspected, and why.
    Uninspectable(PathBuf, String),
}

/// Reports which assets a launch would supply, for the catalog check.
pub(crate) fn present(context: &AppContext) -> Survey {
    let mut tree = Vec::new();
    let mut skills = None;
    for asset in declared(context) {
        match (SystemFileSystem::look(&asset.path), asset.source) {
            (Ok(Some(_)), Source::Child) => skills = Some(asset.path),
            (Ok(Some(_)), Source::Tree) => tree.push(asset.name),
            (Ok(None), _) => {}
            (Err(error), _) => return Survey::Uninspectable(asset.path, error.to_string()),
        }
    }
    Survey::Held { tree, skills }
}

/// Returns how many names the wrapper's own tree may hold.
///
/// The tree's own count, not the declared total: the message it serves names
/// the tree and nothing else, and reporting ten there would tell a reader that
/// a directory holding nine possible names is missing all ten.
pub(crate) const fn tree_count() -> usize {
    TREE_ASSETS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The set is the child's published user-scope tree, minus the one name
    /// supplied from the child's own directory. A name added without a reason
    /// is the thing this pins against.
    #[test]
    fn the_tree_set_is_the_published_tree_without_the_native_one() {
        assert_eq!(
            TREE_ASSETS,
            &[
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
        assert!(!TREE_ASSETS.contains(&NATIVE));
        // The tree's own count, which the empty-tree message reports. Ten
        // names are declared; nine of them can be in the tree.
        assert_eq!(tree_count(), 9);
    }

    /// The child validates a marketplace's install location by string prefix,
    /// so a linked plugins directory reports as corrupted.
    #[test]
    fn the_plugin_tree_is_never_supplied() {
        assert!(!TREE_ASSETS.contains(&"plugins"));
        assert_ne!(NATIVE, "plugins");
    }
}
