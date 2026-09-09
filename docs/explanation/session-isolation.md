# Session isolation

`claude-session` separates three scopes. An account supplies durable native identity, authentication, and the project tree every run of it shares. A running agent supplies the child state directory that one run writes into. A profile supplies the composed settings the child is launched with, and their provenance. This page explains that split.

Exact paths, modes, and writers are in [XDG storage](../reference/xdg-storage.md). Launch injection is in [the wrapper model](./wrapper-model.md).

## What a session is

A wrapper session is an account, an agent, and a profile:

- The account selects one stored authentication mode, one credential store, and one projects tree.
- The agent selects one child state directory, which is what `CLAUDE_CONFIG_DIR` points at.
- The profile selects one composed `settings.json` and its provenance sidecar.

The child exclusively owns its saved login and its native state. The wrapper passes the profile's composed document through the native `--settings` flag. The account and profile selections are related rather than independent: an account carries [the profile it runs with](../reference/accounts.md#the-bound-profile), and that binding is one rung of the profile ladder. An explicit flag, the environment, or a project file still overrides it, and an account created before the binding existed resolves its profile from configuration alone.

Sessions are not conversations. Two agents of one account share the login and the project tree, including the durable per-project memory the child writes there. A caller needing conversation separation uses the child's own session identifier.

## Why the agent is a scope

`CLAUDE_CONFIG_DIR` relocates every path the child reads, at once. Inside it, almost everything the child writes is already keyed by something: transcripts by session identifier, session registrations by process id, per-session environments by session identifier. Three files are keyed by nothing — the child's own `.claude.json`, `history.jsonl`, and `.credentials.json` — and two agents of one account interleave their writes to them.

The first two are split by giving each agent its own directory ([ADR-0113](../decisions/ADR-0113-key-a-session-to-its-running-agent.md)). The third must not be: the child rotates and revokes on refresh and coordinates that across processes through one file, so N copies is the failure [ADR-0025](../decisions/ADR-0025-share-one-native-login-per-account.md) exists to prevent. It stays at the account and is reached through the child's own credential-store variable ([ADR-0104](../decisions/ADR-0104-share-one-credential-store.md)), which relocates it independently.

Splitting at directory granularity instead would take `projects/` with it, and durable per-project memory lives under that. A wrapper of this same shape was observed doing exactly that: one machine accumulated fifty-two memory directories, two of them for a single repository, neither able to read the other. So the tree stays at the account and each session directory reaches it through a declared link ([ADR-0103](../decisions/ADR-0103-permit-a-declared-link.md)).

Which agent a session belongs to is read from the process the wrapper is about to exec into: the wrapper becomes the child, so its own identifier and start time name the agent and answer for it afterwards. [Sessions](../reference/sessions.md#what-a-session-is) has the exact read.

A session lasts exactly one agent run, so the directory is new every launch and nothing inside it carries over: the child's prompt history and the keys it writes start empty, while the transcripts and the peer registry are shared trees the session links to instead.

The agent keys the child's state directory and nothing else. It does not key composed settings, which is the error [why the profile is the key](#why-the-profile-is-the-key) describes.

## Why the agent name is not enough on its own

A process identifier names one process only inside the namespace that issued it. Bind-mounting the state tree into containers — which is what makes one saved login serve every container, and is wanted — crosses that boundary: each container numbers its own processes, so one identifier means one agent on the host and a different one inside. The names stop discriminating exactly where the state is shared, which is the worst place for it, because the directories still look separate.

So the namespace that issued the identifier is the path component above it ([ADR-0107](../decisions/ADR-0107-scope-a-terminal-to-its-namespace.md), [ADR-0113](../decisions/ADR-0113-key-a-session-to-its-running-agent.md)). It is the process namespace, because that is what issues process identifiers and what `/proc` reports them against. One namespace rather than a ladder of them: the agent is named by a single fact, so a single namespace scopes it.

A run whose process namespace cannot be read names no agent, and the launch refuses rather than proceed. The alternative — naming the agent anyway — would restore the collision knowingly.

The namespace alone still cannot tell two kernels apart: namespace inodes are per-kernel counters with fixed initial values, so a virtual machine reaching this tree over a filesystem share reports the same links as the host. The component therefore fingerprints the kernel's own identity together with the link — the machine identifier, or the boot identifier where the machine carries none, and a kernel naming neither makes the rung unavailable as above ([ADR-0109](../decisions/ADR-0109-discriminate-namespaces-across-kernels.md)).

## Why the peer registry is shared

The child discovers its peer sessions by reading pid-keyed registrations under its configuration directory's `sessions/` name, and messages them over per-boot sockets. That directory is in the already-keyed class from the section above — registrations interleave nothing — so splitting it per session was collateral damage of the all-or-nothing relocation, and it silently cost every session the ability to see the others. Each session directory's `sessions` name is therefore a declared link ([ADR-0103](../decisions/ADR-0103-permit-a-declared-link.md)) to one registry at `peers/<boot>/<namespace>/` in the state root, shared across accounts: awareness across accounts is wanted, and the storage threat model is accident rather than this user's own processes ([ADR-0108](../decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md)).

The scope's two components answer whether a listed peer is reachable. The mount namespace carries both the registry files and the sockets registrations name, so two containers of one kernel — which share a boot — stay apart. The boot identifier separates kernels, which the namespace component cannot. The child lists only inside that scope, where listing and messaging coincide. The wrapper's report crosses the boundary by reading each session's declared registry, while messaging does not; its `reachable` field says whether the row's registry is this run's scope.

The wrapper shares the directory and reads only the registration facts its report needs under [ADR-0114](../decisions/ADR-0114-name-a-reported-session-as-the-child-does.md) and [ADR-0124](../decisions/ADR-0124-describe-a-reported-session-with-the-children-facts.md). It never writes, ages, or interprets the child's status. A launch that cannot derive the scope, or cannot place the link, still degrades to the private registry a session would otherwise have.

## Mounting the trees into an isolated environment

Container and virtual-machine setups that bind the wrapper's trees select the sharing they get from the scopes above; the wrapper needs no configuration for it. What crosses, and how:

| Tree                       | Mode       | Why                                                                    |
| -------------------------- | ---------- | ---------------------------------------------------------------------- |
| Config (`claude-session/`) | read-only  | User-authored; the wrapper never writes it                             |
| State (`claude-session/`)  | read-write | Accounts, sessions, composed settings, and the peer registry live here |
| Data (`claude-session/`)   | read-only  | The asset tree; supplied to sessions, never written                    |

Mount whole directories rather than single files: a file mount conveys an inode, and the wrapper and child both replace files by rename, which a file mount makes invisible until remount. Mount the state tree whole rather than narrowing it to `accounts/`: the peer registry sits beside it, and a narrowed mount silently costs the environment its session awareness.

Inside one environment the scopes do the right thing without help. Sessions of one container share its registry and a separate-kernel guest derives its own. The wrapper can report sessions across those scopes, but only rows whose registry equals this run's are reachable; the child's sockets remain private to their scope.

## Why the profile is the key

The composed document is a pure function of the profile, its ordered pieces, and their contents. Nothing about the agent, the working directory, or the project enters it. Keying it by any of those is the error of keying a build artifact by who ran the build: two runs that should share an artifact get two, and two runs that should not share one get one.

Two profiles launched side by side are the case that decides it. They must never meet, and under an input-addressed key they cannot: each names its own entry, and an entry is written once and never rewritten. The converse holds too — identical inputs from different sessions, or from different accounts, name one entry, which is correct because the bytes are identical. A wrapper of this same shape was observed keying composed child configuration by terminal instead, so that a second profile in one terminal overwrote the first and unlinked files a live child depended on; that is the failure this split exists to make unrepresentable.

Whole-root-per-profile isolation was rejected: it would split the login, history, and trust that one account exists to share. Exact names and the write rule are in [XDG storage](../reference/xdg-storage.md#composed-settings-entries); why, in [ADR-0064](../decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md).

## Why both scopes are state

Account config and composed settings are durable program-written state that may survive reboot. They are not:

- user-authored Configuration;
- safely disposable Cache.

The Runtime base is unused: the wrapper opens no socket, and its [write locks](../reference/xdg-storage.md#lock-scopes) live beside the files they guard rather than in a separate tree ([ADR-0060](../decisions/ADR-0060-lock-the-writes-that-are-not-derivable.md)). Durable state is never relocated to it or to a shared temporary directory.

## One writer per artifact

- The child writes account `config/`, including its saved login, projects, history, and trust state. The one exception is a single key in its `.claude.json`, which a login writes so the child's first-run setup does not stand between an authenticated account and its prompt ([ADR-0098](../decisions/ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md)).
- The account subsystem writes `auth-mode.json`, any local OAuth token, and the last-used marker.
- The session subsystem writes the declared links and, beside each session directory, the witness record a launch leaves so a later run can judge liveness ([ADR-0110](../decisions/ADR-0110-record-the-terminal-witness-at-launch.md), [ADR-0113](../decisions/ADR-0113-key-a-session-to-its-running-agent.md)).
- The composition subsystem writes composed settings and their provenance.

The wrapper never reads, copies, fingerprints, or synchronizes the child credential. Wrapper-owned files use atomic write-then-rename; directory creation is idempotent.

## Security posture

Wrapper-managed directories are private, non-symlink, and current-user-owned. Checks run on every invocation rather than relying on creation-time state. Wrapper-owned secrets and metadata are private and atomically replaced.

What those checks defend against is accident — drift, a restored backup, a sync tool — and not a process running as this user, which needs no race to read a credential it already has access to ([ADR-0061](../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md)). That boundary is why validation is a non-following metadata pass rather than a confined traversal.

The child-owned credential is validated only as a path when presence matters. The wrapper does not read, chmod, rewrite, or emit it. Cleanup never follows symbolic links.

## Cleanup

A launch removes that account's sessions whose agent has exited, because one directory per agent run would otherwise accumulate; nothing else is collected without being asked, and composed settings entries stay immutable and permanent. What a user asks for beyond that is `session clean`, which judges every session directory against its recorded witness and removes every one this run cannot prove is live, after confirming. The tree is the wrapper's own, so a directory in it is a session it can account for or it is garbage, with no third state left to accumulate ([ADR-0112](../decisions/ADR-0112-keep-only-the-session-proven-live.md)). Exact verdicts and the verb's grammar are in [sessions](../reference/sessions.md). Explicit account removal still deletes the whole account tree — and leaves composed settings alone, because they are not account state.

## Further reading

- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
- [`directories`](https://docs.rs/directories/)
- [`xdg-ninja`](https://github.com/b3nj5m1n/xdg-ninja)
