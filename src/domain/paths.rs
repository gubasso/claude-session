//! Absolute XDG-owned application paths.

use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use crate::{domain::identifier::Identifier, error::DomainError};

/// The directory name every XDG base is namespaced under.
///
/// The project's own name, which is what the wrapper answers to everywhere
/// ([ADR-0115](../../docs/decisions/ADR-0115-return-to-the-project-namespace.md)).
/// One constant, so the four bases can only move together.
const NAMESPACE: &str = "claude-session";

/// The child's own user-scope configuration directory, relative to `$HOME`.
///
/// Carried against the launch obligation of [ADR-0089]: the wrapper supplies
/// the skill directory from here, and cannot name it without the child's own
/// spelling ([ADR-0125]).
///
/// [ADR-0089]: ../../docs/decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md
/// [ADR-0125]: ../../docs/decisions/ADR-0125-supply-skills-from-the-native-directory.md
const CHILD_CONFIG_DIR: &str = ".claude";

/// Absolute XDG namespace paths owned by the wrapper.
#[derive(Clone, Debug)]
pub(crate) struct XdgPaths {
    config: PathBuf,
    state: PathBuf,
    data: PathBuf,
    cache: PathBuf,
    /// The child's native configuration directory, under `$HOME`.
    child_config: PathBuf,
}

impl XdgPaths {
    /// Resolves all four bases from an OS-string environment snapshot.
    ///
    /// `$HOME` is required, and required to be absolute, whether or not a base
    /// falls back to it. Before [ADR-0125] it was needed only for a fallback,
    /// so a run with four absolute bases could resolve without one. That run
    /// can no longer be served: the skill source is `$HOME` joined with the
    /// child's own directory, and a relative or empty value would make it a
    /// path read against the working directory and then stored, verbatim, as
    /// the target of a link inside a session. One required value beats the
    /// three branches that serving the other case would need
    /// ([ADR-0048](../../docs/decisions/ADR-0048-build-for-a-present-need.md)).
    ///
    /// [ADR-0125]: ../../docs/decisions/ADR-0125-supply-skills-from-the-native-directory.md
    pub(crate) fn resolve(environment: &[(OsString, OsString)]) -> Result<Self, DomainError> {
        let map: HashMap<&OsStr, &OsStr> = environment
            .iter()
            .map(|(key, value)| (key.as_os_str(), value.as_os_str()))
            .collect();
        let home = map
            .get(OsStr::new("HOME"))
            .map(|value| PathBuf::from(*value))
            .filter(|home| home.is_absolute())
            .ok_or(DomainError::MissingHome)?;
        let base = |name: &str, fallback: &str| -> PathBuf {
            if let Some(value) = map.get(OsStr::new(name)) {
                let path = PathBuf::from(value);
                if !value.is_empty() && path.is_absolute() {
                    return path;
                }
            }
            home.join(fallback)
        };
        Ok(Self {
            config: base("XDG_CONFIG_HOME", ".config").join(NAMESPACE),
            state: base("XDG_STATE_HOME", ".local/state").join(NAMESPACE),
            data: base("XDG_DATA_HOME", ".local/share").join(NAMESPACE),
            cache: base("XDG_CACHE_HOME", ".cache").join(NAMESPACE),
            child_config: home.join(CHILD_CONFIG_DIR),
        })
    }

    /// Returns the configuration namespace.
    pub(crate) fn config(&self) -> &Path {
        &self.config
    }
    /// Returns the state namespace.
    pub(crate) fn state(&self) -> &Path {
        &self.state
    }
    // Resolving all four namespaces is the XDG contract itself, so these two are
    // carried without a reader rather than dropped. The allow is scoped to the
    // item so a genuinely dead addition elsewhere still reports.
    /// Returns the data namespace.
    #[allow(dead_code, reason = "the asset tree is the base's only artifact")]
    pub(crate) fn data(&self) -> &Path {
        &self.data
    }
    /// Returns the cache namespace.
    #[allow(dead_code, reason = "no namespace has a discardable artifact yet")]
    pub(crate) fn cache(&self) -> &Path {
        &self.cache
    }

    /// Returns the user's machine-local tree of child assets.
    ///
    /// Data rather than State: it is user-authored content, portable between
    /// machines, and the wrapper never writes it
    /// ([ADR-0106](../../docs/decisions/ADR-0106-supply-child-assets-from-one-tree.md)).
    pub(crate) fn assets(&self) -> PathBuf {
        self.data.join("assets")
    }
    /// Returns the skill directory the child reads when nothing relocates it.
    ///
    /// Outside every XDG base and outside the wrapper's managed region, because
    /// it is the child's own published location and the wrapper only reads it.
    /// Skills are the one asset with installers of their own, and an installer
    /// writes this path; supplying them from here is what lets it stay an
    /// ordinary directory
    /// ([ADR-0125](../../docs/decisions/ADR-0125-supply-skills-from-the-native-directory.md)).
    pub(crate) fn child_skills(&self) -> PathBuf {
        self.child_config.join("skills")
    }
    /// Returns the user's machine-local read-only tree of child plugins.
    ///
    /// Beside the asset tree rather than inside it: that set is the child's
    /// published user-scope names and this is not one of them, which is also
    /// why `plugins` stays absent from it
    /// ([ADR-0117](../../docs/decisions/ADR-0117-supply-plugins-from-a-read-only-seed.md)).
    pub(crate) fn plugin_seed(&self) -> PathBuf {
        self.data.join("plugin-seed")
    }
    /// Returns the accounts collection directory.
    pub(crate) fn accounts(&self) -> PathBuf {
        self.state.join("accounts")
    }
    /// Returns one account's directory.
    pub(crate) fn account(&self, account: &Identifier) -> PathBuf {
        self.accounts().join(account.as_str())
    }
    /// Returns the child-owned native configuration directory for one account.
    ///
    /// The wrapper creates it; the child writes everything inside it.
    pub(crate) fn account_config(&self, account: &Identifier) -> PathBuf {
        self.account(account).join("config")
    }
    /// Returns the collection of one account's per-terminal state directories.
    pub(crate) fn account_sessions(&self, account: &Identifier) -> PathBuf {
        self.account(account).join("sessions")
    }
    /// Returns the namespace directory one account's terminals are grouped by.
    ///
    /// A terminal name is unique only inside the namespace that issued it, so
    /// the namespace is a component of its own rather than part of the name
    /// ([ADR-0107](../../docs/decisions/ADR-0107-scope-a-terminal-to-its-namespace.md)).
    pub(crate) fn account_namespace(
        &self,
        account: &Identifier,
        namespace: &Identifier,
    ) -> PathBuf {
        self.account_sessions(account).join(namespace.as_str())
    }
    /// Returns the child state directory for one account and one terminal.
    ///
    /// This is what `CLAUDE_CONFIG_DIR` points at. It is per terminal because
    /// three of the files the child writes there are keyed by nothing and
    /// interleave between panes
    /// ([ADR-0102](../../docs/decisions/ADR-0102-key-child-state-by-terminal.md)).
    pub(crate) fn account_session(
        &self,
        account: &Identifier,
        namespace: &Identifier,
        terminal: &Identifier,
    ) -> PathBuf {
        self.account_namespace(account, namespace)
            .join(terminal.as_str())
    }
    /// Returns the projects tree every terminal of one account shares.
    ///
    /// Inside `config/` rather than beside it, so an account that predates the
    /// split keeps the tree the child already filled, and durable per-project
    /// memory is not divided per terminal.
    pub(crate) fn account_projects(&self, account: &Identifier) -> PathBuf {
        self.account_config(account).join("projects")
    }
    /// Returns the child's own configuration file inside one session directory.
    pub(crate) fn session_native_config(
        &self,
        account: &Identifier,
        namespace: &Identifier,
        terminal: &Identifier,
    ) -> PathBuf {
        self.account_session(account, namespace, terminal)
            .join(".claude.json")
    }
    /// Returns the shared-projects link inside one session directory.
    pub(crate) fn session_projects_link(
        &self,
        account: &Identifier,
        namespace: &Identifier,
        terminal: &Identifier,
    ) -> PathBuf {
        self.account_session(account, namespace, terminal)
            .join("projects")
    }
    /// Returns the root every peer-registry scope lives under.
    ///
    /// Its own path so a stale registry link — one whose target is inside
    /// this root but names an earlier boot's scope — can be recognised as
    /// the wrapper's own writing rather than a foreign symlink.
    pub(crate) fn peers(&self) -> PathBuf {
        self.state.join("peers")
    }
    /// Returns the peer registry one boot-and-namespace scope shares.
    ///
    /// At the state root rather than under an account, because the registry
    /// is host-wide: awareness across accounts is the point, and the records
    /// are the child's own pid-keyed registrations ([ADR-0108]).
    ///
    /// [ADR-0108]: ../../docs/decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md
    pub(crate) fn peer_registry(&self, boot: &Identifier, namespace: &Identifier) -> PathBuf {
        self.peers().join(boot.as_str()).join(namespace.as_str())
    }
    /// Re-anchors a registry a session's own link names under this run's peers
    /// root.
    ///
    /// The target is never opened. Its last three components are read, and the
    /// path this run then builds is one of its own ([ADR-0123]).
    ///
    /// [ADR-0123]: ../../docs/decisions/ADR-0123-read-a-sessions-own-peer-registry.md
    pub(crate) fn adopted_registry(&self, target: &Path) -> Option<PathBuf> {
        let components: Vec<_> = target.components().collect();
        let [.., peers, boot, namespace] = components.as_slice() else {
            return None;
        };
        let std::path::Component::Normal(peers) = peers else {
            return None;
        };
        let std::path::Component::Normal(boot) = boot else {
            return None;
        };
        let std::path::Component::Normal(namespace) = namespace else {
            return None;
        };
        if *peers != OsStr::new("peers") {
            return None;
        }
        let boot: Identifier = boot.to_str()?.parse().ok()?;
        let namespace: Identifier = namespace.to_str()?.parse().ok()?;
        Some(self.peer_registry(&boot, &namespace))
    }
    /// Returns the witness marker recorded beside one session directory.
    ///
    /// Beside rather than inside, following the lock's precedent: the record
    /// must survive the directory it judges, and the child owns every name
    /// inside its own state directory
    /// ([ADR-0110](../../docs/decisions/ADR-0110-record-the-terminal-witness-at-launch.md)).
    /// The leading dot keeps it out of terminal discovery twice over: the walk
    /// takes directories only, and an identifier cannot begin with one.
    pub(crate) fn session_witness(
        &self,
        account: &Identifier,
        namespace: &Identifier,
        terminal: &Identifier,
    ) -> PathBuf {
        self.account_namespace(account, namespace)
            .join(format!(".{}.witness.json", terminal.as_str()))
    }
    /// Returns the peer-registry link inside one session directory.
    ///
    /// The name is the child's own `sessions`, which is what makes the child
    /// read the shared registry without knowing the wrapper exists.
    pub(crate) fn session_registry_link(
        &self,
        account: &Identifier,
        namespace: &Identifier,
        terminal: &Identifier,
    ) -> PathBuf {
        self.account_session(account, namespace, terminal)
            .join("sessions")
    }
    /// Returns one account's authentication-mode metadata.
    pub(crate) fn account_auth_mode(&self, account: &Identifier) -> PathBuf {
        self.account(account).join("auth-mode.json")
    }
    /// Returns one account's profile binding.
    ///
    /// Beside the authentication metadata rather than inside it, so rebinding
    /// never writes the file the token rotation sequence commits
    /// ([ADR-0096](../../docs/decisions/ADR-0096-bind-a-profile-to-an-account.md)).
    pub(crate) fn account_profile(&self, account: &Identifier) -> PathBuf {
        self.account(account).join("profile.json")
    }
    /// Returns one account's wrapper-owned token path.
    pub(crate) fn account_oauth_token(&self, account: &Identifier) -> PathBuf {
        self.account(account).join("oauth-token")
    }
    /// Returns one account's credential lock file.
    ///
    /// Beside the account rather than inside it, so removal cannot destroy the
    /// inode that excludes a concurrent login from the tree being removed
    /// ([ADR-0087](../../docs/decisions/ADR-0087-keep-the-credential-lock-beside-the-account.md)).
    /// The leading dot keeps it out of discovery twice over: the walk takes
    /// directories only, and an identifier cannot begin with one.
    ///
    /// The sentinel is a lock handle rather than a claim, so it carries no
    /// security check and is never swept; see the lock scopes in XDG storage.
    pub(crate) fn account_credentials_lock(&self, account: &Identifier) -> PathBuf {
        self.accounts().join(format!(".{}.lock", account.as_str()))
    }
    /// Returns the child-owned saved-login path without opening it.
    pub(crate) fn account_credentials(&self, account: &Identifier) -> PathBuf {
        self.account_config(account).join(".credentials.json")
    }
    /// Returns the last-used account marker.
    pub(crate) fn last_account(&self) -> PathBuf {
        self.state.join("state").join("last-account")
    }
    /// Returns the composed-settings store directory.
    pub(crate) fn composed(&self) -> PathBuf {
        self.state.join("composed")
    }
    /// Returns the directory holding one profile document per profile.
    pub(crate) fn profiles(&self) -> PathBuf {
        self.config.join("profiles")
    }
    /// Returns the profile document for one profile name.
    pub(crate) fn profile_file(&self, profile: &Identifier) -> PathBuf {
        self.profiles().join(format!("{}.yaml", profile.as_str()))
    }
    /// Returns the settings piece file for one piece name.
    pub(crate) fn piece_file(&self, piece: &Identifier) -> PathBuf {
        self.config
            .join("settings")
            .join(format!("{}.json", piece.as_str()))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn fixture() -> XdgPaths {
        XdgPaths::resolve(&[
            (OsString::from("HOME"), OsString::from("/h")),
            (OsString::from("XDG_CONFIG_HOME"), OsString::from("/c")),
            (OsString::from("XDG_STATE_HOME"), OsString::from("/s")),
            (OsString::from("XDG_DATA_HOME"), OsString::from("/d")),
            (OsString::from("XDG_CACHE_HOME"), OsString::from("/k")),
        ])
        .expect("absolute bases resolve")
    }

    /// The skill source is `$HOME` joined with the child's own directory, and a
    /// launch stores it verbatim as a link target inside a session. A relative
    /// or empty value would therefore be read against the working directory and
    /// then persisted, so resolution refuses one rather than carrying it
    /// (ADR-0125).
    ///
    /// Four absolute bases is the case that makes this a rule of its own: every
    /// base resolves without ever consulting `$HOME`, so nothing else in the
    /// type would notice the value is unusable.
    #[test]
    fn an_unusable_home_is_refused_even_when_every_base_resolves() {
        for home in ["", "relative/home", "~/home"] {
            let resolved = XdgPaths::resolve(&[
                (OsString::from("HOME"), OsString::from(home)),
                (OsString::from("XDG_CONFIG_HOME"), OsString::from("/c")),
                (OsString::from("XDG_STATE_HOME"), OsString::from("/s")),
                (OsString::from("XDG_DATA_HOME"), OsString::from("/d")),
                (OsString::from("XDG_CACHE_HOME"), OsString::from("/k")),
            ]);
            assert!(
                matches!(resolved, Err(DomainError::MissingHome)),
                "HOME={home:?} is not an absolute path, so resolution must refuse it"
            );
        }
        let absent = XdgPaths::resolve(&[
            (OsString::from("XDG_CONFIG_HOME"), OsString::from("/c")),
            (OsString::from("XDG_STATE_HOME"), OsString::from("/s")),
            (OsString::from("XDG_DATA_HOME"), OsString::from("/d")),
            (OsString::from("XDG_CACHE_HOME"), OsString::from("/k")),
        ]);
        assert!(matches!(absent, Err(DomainError::MissingHome)));
    }

    /// The child's own directory is where a skill installer writes, so it comes
    /// from `$HOME` rather than from any XDG base (ADR-0125).
    #[test]
    fn the_skill_source_is_the_native_directory() {
        assert_eq!(fixture().child_skills(), Path::new("/h/.claude/skills"));
    }

    /// A compiler-checked copy of the artifact table's path column, so a
    /// renamed directory fails here rather than silently relocating a user's
    /// durable state.
    #[test]
    fn every_managed_path_matches_the_artifact_table() {
        let paths = fixture();
        let work = "work".parse().expect("identifier");
        assert_eq!(paths.accounts(), Path::new("/s/claude-session/accounts"));
        assert_eq!(
            paths.account(&work),
            Path::new("/s/claude-session/accounts/work")
        );
        assert_eq!(
            paths.account_config(&work),
            Path::new("/s/claude-session/accounts/work/config")
        );
        assert_eq!(
            paths.account_auth_mode(&work),
            Path::new("/s/claude-session/accounts/work/auth-mode.json")
        );
        assert_eq!(
            paths.account_profile(&work),
            Path::new("/s/claude-session/accounts/work/profile.json")
        );
        assert_eq!(
            paths.account_oauth_token(&work),
            Path::new("/s/claude-session/accounts/work/oauth-token")
        );
        assert_eq!(
            paths.account_credentials_lock(&work),
            Path::new("/s/claude-session/accounts/.work.lock")
        );
        assert_eq!(
            paths.account_credentials(&work),
            Path::new("/s/claude-session/accounts/work/config/.credentials.json")
        );
        let pane = "pts-3".parse().expect("terminal identifier");
        let space = "mnt-1a2b3c4d".parse().expect("namespace identifier");
        assert_eq!(
            paths.account_sessions(&work),
            Path::new("/s/claude-session/accounts/work/sessions")
        );
        assert_eq!(
            paths.account_namespace(&work, &space),
            Path::new("/s/claude-session/accounts/work/sessions/mnt-1a2b3c4d")
        );
        assert_eq!(
            paths.account_session(&work, &space, &pane),
            Path::new("/s/claude-session/accounts/work/sessions/mnt-1a2b3c4d/pts-3")
        );
        assert_eq!(
            paths.account_projects(&work),
            Path::new("/s/claude-session/accounts/work/config/projects")
        );
        assert_eq!(
            paths.session_native_config(&work, &space, &pane),
            Path::new("/s/claude-session/accounts/work/sessions/mnt-1a2b3c4d/pts-3/.claude.json")
        );
        assert_eq!(
            paths.session_projects_link(&work, &space, &pane),
            Path::new("/s/claude-session/accounts/work/sessions/mnt-1a2b3c4d/pts-3/projects")
        );
        assert_eq!(
            paths.session_witness(&work, &space, &pane),
            Path::new("/s/claude-session/accounts/work/sessions/mnt-1a2b3c4d/.pts-3.witness.json")
        );
        assert_eq!(
            paths.last_account(),
            Path::new("/s/claude-session/state/last-account")
        );
        assert_eq!(paths.composed(), Path::new("/s/claude-session/composed"));
        assert_eq!(paths.profiles(), Path::new("/c/claude-session/profiles"));
        assert_eq!(
            paths.profile_file(&work),
            Path::new("/c/claude-session/profiles/work.yaml")
        );
        assert_eq!(
            paths.piece_file(&work),
            Path::new("/c/claude-session/settings/work.json")
        );
    }

    #[test]
    fn a_link_into_the_peer_root_re_anchors_under_this_runs_root() {
        let paths = fixture();
        assert_eq!(
            paths.adopted_registry(Path::new("/another/mount/peers/boot-one/mnt-two")),
            Some(PathBuf::from("/s/claude-session/peers/boot-one/mnt-two"))
        );
    }

    #[test]
    fn a_link_that_names_no_registry_re_anchors_nothing() {
        let paths = fixture();
        for target in [
            "/other/peers/boot-only",
            "/other/peers/boot-one/mnt-two/extra",
            "/other/peers/.bad/mnt-two",
            "/other/registries/boot-one/mnt-two",
        ] {
            assert_eq!(paths.adopted_registry(Path::new(target)), None, "{target}");
        }
    }
}
