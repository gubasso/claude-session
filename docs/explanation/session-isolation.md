# Session isolation

`claude-session` separates three scopes. An account supplies durable native identity, authentication, and the project tree every run of it shares. A terminal supplies the child state directory one run writes into. A profile supplies the composed settings the child is launched with, and their provenance. This page explains that split.

Exact paths, modes, and writers are in [XDG storage](../reference/xdg-storage.md). Launch injection is in [the wrapper model](./wrapper-model.md).

## What a session is

A wrapper session is an account, a terminal, and a profile:

- The account selects one stored authentication mode, one credential store, and one projects tree.
- The terminal selects one child state directory, which is what `CLAUDE_CONFIG_DIR` points at.
- The profile selects one composed `settings.json` and its provenance sidecar.

The child exclusively owns its saved login and its native state. The wrapper passes the profile's composed document through the native `--settings` flag. The account and profile selections are related rather than independent: an account carries [the profile it runs with](../reference/accounts.md#the-bound-profile), and that binding is one rung of the profile ladder. An explicit flag, the environment, or a project file still overrides it, and an account created before the binding existed resolves its profile from configuration alone.

Sessions are not conversations. Two terminals of one account share the login and the project tree, including the durable per-project memory the child writes there. A caller needing conversation separation uses the child's own session identifier.

## Why the terminal is a scope

`CLAUDE_CONFIG_DIR` relocates every path the child reads, at once. Inside it, almost everything the child writes is already keyed by something: transcripts by session identifier, session registrations by process id, per-session environments by session identifier. Three files are keyed by nothing — the child's own `.claude.json`, `history.jsonl`, and `.credentials.json` — and two panes of one account interleave their writes to them.

The first two are split by giving each terminal its own directory ([ADR-0102](../decisions/ADR-0102-key-child-state-by-terminal.md)). The third must not be: the child rotates and revokes on refresh and coordinates that across processes through one file, so N copies is the failure [ADR-0025](../decisions/ADR-0025-share-one-native-login-per-account.md) exists to prevent. It stays at the account and is reached through the child's own credential-store variable ([ADR-0104](../decisions/ADR-0104-share-one-credential-store.md)), which relocates it independently.

Splitting at directory granularity instead would take `projects/` with it, and durable per-project memory lives under that. A wrapper of this same shape was observed doing exactly that: one machine accumulated fifty-two memory directories, two of them for a single repository, neither able to read the other. So the tree stays at the account and each session directory reaches it through a declared link ([ADR-0103](../decisions/ADR-0103-permit-a-declared-link.md)).

The terminal keys the child's state directory and nothing else. It does not key composed settings, which is the error the next section describes.

## Why the terminal is not enough on its own

A terminal name answers "which pane" only inside the namespace that issued it. Bind-mounting the state tree into containers — which is what makes one saved login serve every container, and is wanted — crosses that boundary: each container carries its own devpts, so the first pane in each is `/dev/pts/0` and every one of them derives `pts-0`. The names stop discriminating exactly where the state is shared, which is the worst place for it, because the directories still look separate.

So the namespace that issued the name is the path component above it ([ADR-0107](../decisions/ADR-0107-scope-a-terminal-to-its-namespace.md)). It is read per rung rather than once, because the two rungs read identifiers the kernel scopes differently: the mount namespace carries the devpts instance that issued a device name, and the process namespace issues the session ids the second rung reads. A container run with the host's process namespace is the configuration where one uniform choice would collide and these do not.

A rung whose namespace cannot be read names nothing, so the ladder falls past it and the existing refusal absorbs the case. The alternative — naming the terminal anyway — would restore the collision knowingly.

The namespace alone still cannot tell two kernels apart: namespace inodes are per-kernel counters with fixed initial values, so a virtual machine reaching this tree over a filesystem share reports the same links as the host. The component therefore fingerprints the kernel's own identity together with the link — the machine identifier, or the boot identifier where the machine carries none, and a kernel naming neither makes the rung unavailable as above ([ADR-0109](../decisions/ADR-0109-discriminate-namespaces-across-kernels.md)).

## Why the peer registry is shared

The child discovers its peer sessions by reading pid-keyed registrations under its configuration directory's `sessions/` name, and messages them over per-boot sockets. That directory is in the already-keyed class from the section above — registrations interleave nothing — so splitting it per terminal was collateral damage of the all-or-nothing relocation, and it silently cost every session the ability to see the others. Each session directory's `sessions` name is therefore a declared link ([ADR-0103](../decisions/ADR-0103-permit-a-declared-link.md)) to one registry at `peers/<boot>/<namespace>/` in the state root, shared across accounts: awareness across accounts is wanted, and the storage threat model is accident rather than this user's own processes ([ADR-0108](../decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md)).

The scope's two components answer whether a listed peer is reachable. The mount namespace carries both the registry files and the sockets registrations name, so two containers of one kernel — which share a boot — stay apart. The boot identifier separates kernels, which the namespace component cannot: every kernel names its initial mount namespace with the same fixed inode, so a virtual machine reaching this tree over a filesystem share would otherwise land in the host's scope while its sockets stay unreachable. Two sessions list each other exactly when they could also message each other, and a peer in another scope is invisible rather than listed and dead.

The wrapper shares the directory and stops. It never reads, parses, or ages a registration — the child owns that schema, and no delegation obligation covers consuming it ([ADR-0089](../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)). A launch that cannot derive the scope, or cannot place the link, degrades to the private registry the terminal always had, because awareness is additive and must never stop an exec.

## Mounting the trees into an isolated environment

Container and virtual-machine setups that bind the wrapper's trees select the sharing they get from the scopes above; the wrapper needs no configuration for it. What crosses, and how:

| Tree                          | Mode       | Why                                                                    |
| ----------------------------- | ---------- | ---------------------------------------------------------------------- |
| Config (`claude-session-rs/`) | read-only  | User-authored; the wrapper never writes it                             |
| State (`claude-session-rs/`)  | read-write | Accounts, sessions, composed settings, and the peer registry live here |
| Data (`claude-session-rs/`)   | read-only  | The asset tree; supplied to sessions, never written                    |

Mount whole directories rather than single files: a file mount conveys an inode, and the wrapper and child both replace files by rename, which a file mount makes invisible until remount. Mount the state tree whole rather than narrowing it to `accounts/`: the peer registry sits beside it, and a narrowed mount silently costs the environment its session awareness.

Inside one environment the scopes do the right thing without help. Sessions of one container share that container's registry; a separate-kernel guest derives its own boot component and keeps its own; and none of them see the host's sessions, whose sockets they could not reach anyway. This is the same honesty the terminal namespace buys for session directories, applied to discovery.

## Why the profile is the key

The composed document is a pure function of the profile, its ordered pieces, and their contents. Nothing about the terminal, the working directory, or the project enters it. Keying it by any of those is the error of keying a build artifact by who ran the build: two runs that should share an artifact get two, and two runs that should not share one get one.

Two profiles launched from one terminal are the case that decides it. They must never meet, and under an input-addressed key they cannot: each names its own entry, and an entry is written once and never rewritten. The converse holds too — identical inputs from different terminals, or from different accounts, name one entry, which is correct because the bytes are identical. A wrapper of this same shape was observed keying composed child configuration by terminal instead, so that a second profile in one terminal overwrote the first and unlinked files a live child depended on; that is the failure this split exists to make unrepresentable.

Whole-root-per-profile isolation was rejected: it would split the login, history, and trust that one account exists to share. Exact names and the write rule are in [XDG storage](../reference/xdg-storage.md#composed-settings-entries); why, in [ADR-0064](../decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md).

## Why both scopes are state

Account config and composed settings are durable program-written state that may survive reboot. They are not:

- user-authored Configuration;
- safely disposable Cache.

The Runtime base is unused: the wrapper opens no socket, and its [write locks](../reference/xdg-storage.md#lock-scopes) live beside the files they guard rather than in a separate tree ([ADR-0060](../decisions/ADR-0060-lock-the-writes-that-are-not-derivable.md)). Durable state is never relocated to it or to a shared temporary directory.

## One writer per artifact

- The child writes account `config/`, including its saved login, projects, history, and trust state. The one exception is a single key in its `.claude.json`, which a login writes so the child's first-run setup does not stand between an authenticated account and its prompt ([ADR-0098](../decisions/ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md)).
- The account subsystem writes `auth-mode.json`, any local OAuth token, and the last-used marker.
- The composition subsystem writes composed settings and their provenance.

The wrapper never reads, copies, fingerprints, or synchronizes the child credential. Wrapper-owned files use atomic write-then-rename; directory creation is idempotent.

## Security posture

Wrapper-managed directories are private, non-symlink, and current-user-owned. Checks run on every invocation rather than relying on creation-time state. Wrapper-owned secrets and metadata are private and atomically replaced.

What those checks defend against is accident — drift, a restored backup, a sync tool — and not a process running as this user, which needs no race to read a credential it already has access to ([ADR-0061](../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md)). That boundary is why validation is a non-following metadata pass rather than a confined traversal.

The child-owned credential is validated only as a path when presence matters. The wrapper does not read, chmod, rewrite, or emit it. Cleanup never follows symbolic links.

## Cleanup

Composed settings entries are immutable and permanent; the wrapper ships no pruning. Automatic cleanup removes nothing but an orphaned atomic temporary, and only explicit account removal deletes an account tree — which leaves composed settings alone, because they are not account state.

## Further reading

- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
- [`directories`](https://docs.rs/directories/)
- [`xdg-ninja`](https://github.com/b3nj5m1n/xdg-ninja)
