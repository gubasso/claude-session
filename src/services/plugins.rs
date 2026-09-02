//! Supplying the child's plugins to every session directory.
//!
//! This module is for pointing one launch at a read-only seed tree and giving
//! the session directory the plugin state that tree already holds. It models
//! nothing either file contains: the bytes are copied and never parsed, which
//! is the whole carry ([ADR-0088]).
//!
//! The seed is not linked in the way assets are. The child records a
//! marketplace's install location as an absolute path and follows it literally,
//! so one tree shared between session directories reports its plugins as
//! uncached — the measurement behind the `plugins` exclusion of [ADR-0106],
//! which this module leaves standing.
//!
//! [ADR-0088]: ../../docs/decisions/ADR-0088-model-nothing-the-child-already-owns.md
//! [ADR-0106]: ../../docs/decisions/ADR-0106-supply-child-assets-from-one-tree.md
//! [ADR-0117]: ../../docs/decisions/ADR-0117-supply-plugins-from-a-read-only-seed.md

use std::path::{Path, PathBuf};

use crate::{
    adapters::filesystem::SystemFileSystem,
    context::AppContext,
    error::AppError,
    services::storage::{atomic, guard},
};

/// The child's plugin state directory, inside a configuration directory.
///
/// A child-owned name carried against the launch obligation of [ADR-0089]: the
/// wrapper writes into the directory the child will read, and no other spelling
/// reaches it. An ordinary English word, so the discovery scan cannot see it and
/// the registration is this comment rather than an entry ([Q-012]).
///
/// [ADR-0089]: ../../docs/decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md
/// [Q-012]: ../../docs/plan/open-questions.md
const PLUGINS: &str = "plugins";

/// The state files a seed supplies, in the order a reader meets them.
///
/// The child registers the seed's marketplaces at startup, but it reconciles
/// the plugins its settings declare before that, so a directory holding neither
/// file loads nothing on its first run. Copying them is what answers the
/// question the directory's newness makes the child ask ([ADR-0117]).
const STATE: &[&str] = &["known_marketplaces.json", "installed_plugins.json"];

/// Returns the seed tree when a launch can supply from it.
///
/// A tree that cannot be inspected is warned about and treated as absent, for
/// the reason the asset service gives: a launch that would work is never
/// refused over a convenience, and the soft `session-plugin-seed` row would
/// otherwise disagree with the guard.
pub(crate) fn locate(context: &AppContext) -> Option<PathBuf> {
    let tree = context.paths().plugin_seed();
    match SystemFileSystem::look(&tree) {
        Ok(Some(facts)) if facts.directory => Some(tree),
        Ok(Some(_)) => {
            tracing::warn!(
                op = "supply_plugins",
                status = "skipped",
                path = %tree.display(),
                "{} is not a directory, so this session starts without plugins",
                tree.display()
            );
            None
        }
        Ok(None) => None,
        Err(error) => {
            tracing::warn!(
                op = "supply_plugins",
                status = "skipped",
                path = %tree.display(),
                "{} could not be inspected, so this session starts without plugins: {}",
                tree.display(),
                error
            );
            None
        }
    }
}

/// Writes the plugin state a seed holds into one session directory.
///
/// A copy rather than a link, for two reasons that both point the same way: the
/// seed is read-only and the child writes its installed set during a run, and a
/// session's copy is discarded with the session rather than becoming a second
/// durable tree of the kind [ADR-0106] rejected.
///
/// A state file the seed does not hold is skipped. A seed carrying only
/// registered marketplaces is a whole answer, and materialising an empty
/// installed set beside it would assert something the user did not.
pub(crate) fn supply(context: &AppContext, session: &Path, seed: &Path) -> Result<(), AppError> {
    let directory = session.join(PLUGINS);
    guard::ensure_directory(context.paths().state(), &directory)?;
    for name in STATE {
        let source = seed.join(name);
        let bytes = match std::fs::read(&source) {
            Ok(bytes) => bytes,
            // Absent is the ordinary case and says nothing. Unreadable is
            // warned about and skipped, because one unreadable file in the
            // user's own tree is not worth refusing a launch over.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                tracing::warn!(
                    op = "supply_plugins",
                    status = "skipped",
                    path = %source.display(),
                    "{} could not be read, so this session starts without it: {}",
                    source.display(),
                    error
                );
                continue;
            }
        };
        atomic::write(&directory.join(name), &bytes, 0o600)?;
    }
    Ok(())
}

/// What the catalog check found when it looked at the seed tree.
pub(crate) enum Survey {
    /// No seed tree, so a launch supplies no plugins and nothing is wrong.
    Absent,
    /// The state files the tree holds, which may be none of them.
    Held(Vec<&'static str>),
    /// The tree could not be inspected, and why.
    Uninspectable(PathBuf, String),
}

/// Reports what a launch would supply, for the catalog check.
///
/// Held means readable as bytes, which is the same question [`supply`] asks and
/// has to be: a check answering "it exists" would pass a directory sitting at a
/// state file's name, or a file the launch cannot open, and then the launch
/// would copy nothing while the report said it would.
pub(crate) fn present(context: &AppContext) -> Survey {
    let tree = context.paths().plugin_seed();
    match SystemFileSystem::look(&tree) {
        Ok(None) => return Survey::Absent,
        Ok(Some(facts)) if !facts.directory => {
            return Survey::Uninspectable(tree, "it is not a directory".to_owned());
        }
        Ok(Some(_)) => {}
        Err(error) => return Survey::Uninspectable(tree, error.to_string()),
    }
    let mut held = Vec::new();
    for name in STATE {
        let source = tree.join(name);
        match std::fs::read(&source) {
            Ok(_) => held.push(*name),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Survey::Uninspectable(source, error.to_string()),
        }
    }
    Survey::Held(held)
}

/// Returns every state file a launch may supply, whether or not a seed holds it.
pub(crate) const fn declared() -> &'static [&'static str] {
    STATE
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pair is what the child needs before it reconciles the plugins its
    /// settings declare. A third name added without a reason is the thing this
    /// pins against.
    #[test]
    fn the_declared_state_is_the_pair_the_child_reads_first() {
        assert_eq!(
            declared(),
            &["known_marketplaces.json", "installed_plugins.json"]
        );
    }

    /// The seat the wrapper writes into is the child's own, so its spelling is
    /// not the wrapper's to choose.
    #[test]
    fn the_state_directory_is_named_by_the_child() {
        assert_eq!(PLUGINS, "plugins");
    }
}
