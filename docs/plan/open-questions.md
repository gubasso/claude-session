# Open questions

## Q-002 — Does the child baseline still guarantee cross-process refresh locking?

Blocks: slice 005 revalidation of the documented minimum, which the login-mode refusal ships against meanwhile because that check fails closed, and the 2026-08-12 dates on the process-runtime and xdg-storage entries, which record the lock and the isolated-directory credential placement as unproven.

Raised: migrated from the perishable child-version and refresh-lock records.

Exit: measurement, revalidate current official release material and run two concurrent processes across refresh without recording credentials. The 2026-08-12 pass did not: renewal fires on the child's schedule, and published fixes above the baseline show that behaviour still moving, so the window has to be observed rather than forced.

## Q-005 — Which page should retire the stale supervised-runtime description?

Blocks: reconciliation of `docs/explanation/architecture.md` with implemented ADR-0084 and the exec-owned process runtime.

Raised: slice 005 found that the non-governing explanation still describes supervision and post-flight marker writes.

Exit: slice revision, assign the current exec model one explanation owner and remove the stale supervised sequence.

## Q-007 — Is a composed entry named by its inputs, or verified against them?

Blocks: whether `docs/reference/xdg-storage.md#composed-settings-entries` keeps reuse as a digest comparison or promotes it to a content check.

Raised: the slice 014 review observed that reuse turns solely on the sidecar's recorded digest matching the one the inputs recompute, and that the digest never ranges over the settings file's own bytes. A sidecar reduced to a correct `digest` field alone is therefore accepted, and a corrupted settings member is reused unread. The implementation matches its owner page verbatim, and the page claims the check makes "two profiles never share settings" a check rather than a probability — it never claims tamper resistance.

Exit: ADR, decide whether the entry invariant is "named by its inputs" or "verified against its inputs", then amend the owner page and the reuse path together. The cost of the second reading is a byte comparison on every launch; the case it defends against presupposes the user corrupting their own mode `0600`, guard-validated state.

## Q-008 — Should an account report distinguish the durable binding from the effective selection as separate fields?

Blocks: whether `account status --json` keeps pairing `profile`, the account's binding, with `profile_source`, the layer that supplied the profile this run resolved.

Raised: the slice 023 review observed that a project file or a flag makes the two describe different profiles, and that the human form now says so in words while the document leaves a reader to infer it. The pairing satisfies slice 022's acceptance, which asks for the bound profile and the provenance of the profile in force, so this is a clarity question rather than a defect. A companion asymmetry is that `account list --json` carries the binding with no provenance at all.

Exit: ADR, decide whether the two facts are one field pair or two, then move the field set and both renderers together. It changes a published document shape, so it needs a slice rather than an edit.

## Q-009 — What excludes a running child from the keys a launch writes?

Blocks: whether [ADR-0098](../decisions/ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md) and [ADR-0105](../decisions/ADR-0105-seed-a-session-at-launch.md) can promise that the wrapper's read-modify-write of the child's `.claude.json` preserves the keys beside it, which [accounts](../reference/accounts.md#what-a-launch-seeds) states as a fact.

Raised: the slice 024 review observed that the credential lock excludes wrapper writers only. A `claude` already running under the account holds nothing, so a trust record it writes between the wrapper's read and its rename is lost. [ADR-0084](../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md) leaves no wrapper process alive to hold a lock for the child's lifetime, so the exclusion the wrapper would need does not exist to be taken.

Narrowed twice since. [ADR-0102](../decisions/ADR-0102-key-child-state-by-terminal.md) gave each terminal its own file, so the two concurrent children that used to race over one document no longer share one; and the slice 029 review moved the wrapper's write behind a change test, so a launch into a directory already carrying both answers does not rewrite the file at all. What is left is the launch that genuinely adds a key — the first in a session directory, or the first from a new workspace — racing a child of the same terminal that is already running.

Exit: measurement, observe whether that residual window is reachable in practice, given it now requires a second launch in a terminal whose child is already live. If it is, the decision that follows is between refusing the write while a child of that terminal is live and accepting the loss, and either one changes what the reference pages may promise.

## Q-010 — Should a saved-login commit witness this run's exchange, or only a credential's presence?

Blocks: whether [accounts](../reference/accounts.md#refresh-token-bootstrap) may promise that a successful refresh-token login exchanged anything, rather than that a credential is there afterwards.

Raised: the slice 026 review observed that `credentials_committable` answers "a safe credential exists" and carries no notion of the operation asking. Against a fresh account that is exact. Against an account that already holds a saved login it is not: a child exiting successfully without exchanging would commit `login` mode on the strength of a file that predates the run, and for a token account that also replaces the stored mode. Slice 027 raised what that costs: the switch now retires the token it supersedes, so a commit taken on a credential that predates the run destroys a working token rather than orphaning one. The gate is shared with the native login, where the same limit has always applied and where a person watched the browser flow that produced the credential; the refresh bootstrap is the first path meant to run unattended, which is what makes the limit worth naming. Nothing in child 2.1.220 reaches the bad state — its exchange branch either completes or exits non-zero — so this is a gap in what the gate can prove rather than an observed failure.

Exit: ADR, decide whether the witness becomes operation-specific and whether that changes the native login too. The comparison that would settle it reads a child-owned credential, which [accounts](../reference/accounts.md#what-an-account-is) forbids and only a `stat` escapes, so the decision is between a weaker witness the wrapper may take, an amendment to that prohibition, and accepting the presence test with the limit documented. Whichever wins needs the regression the review named: an account that starts with a credential, a child that exits successfully without touching it, and an assertion about what the mode becomes.

## Q-011 — What lets a test force one login to interleave with another?

Blocks: whether the lock-held revalidation slice 027 added to `write_login_metadata`, and the guard branch of `retire_superseded`, can carry an acceptance line with a resolved test id rather than a claim.

Raised: the slice 027 review found that retirement turns a previously benign interleaving into the loss of both credentials — a saved-login run judging the child's credential outside the lock, a token rotation committing and retiring inside it, and the first run then writing its mode and retiring the token the second one just wrote. The fix asks the question again under the lock, which is verifiable by reading but not by the harness: every state it distinguishes requires another process to have committed between two points inside one call. The same absence leaves `retire_superseded`'s guard branch untested, since a `config` replaced before the login is caught earlier by `prepare_login`. `CS_TEST_CREATE_CREDENTIAL` shows a test hook in the child is an accepted device, so the shape is available; what it would cost the production path is the open part.

Exit: measurement, decide whether a barrier the harness releases is expressible without a branch in the committed code — a lock the test holds from outside is one candidate, since the wrapper's own `hold` would then block on it with no wrapper change at all. If it is, the two branches get their tests; if it is not, the decision is between a test-only hook and accepting review as the enforcement, labelled honestly.

## Q-012 — Which obligation authorizes carrying the child's credential filename?

Blocks: whether `.credentials.json` and the ten asset names of [ADR-0106](../decisions/ADR-0106-supply-child-assets-from-one-tree.md) belong in [child facts](../reference/child-facts.yaml), and whether the discovery gate should stop being blind to them.

Raised: the slice 027 review observed that the name is carried in `src/domain/paths.rs`, in three reference pages, and now in a wrapper unlink, while [the registry](../reference/child-facts.yaml) holds no entry for it and `tests/repo_contracts/child_facts.rs` scans only the three environment prefixes plus one literal — so the gate passes because it cannot see the name rather than because the carry is registered. The carry predates this slice; what 027 changed is that the wrapper now deletes the path rather than only observing it. None of [ADR-0089](../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)'s three obligations fits cleanly: the wrapper does not launch through the name, no wrapper spelling collides with it, and the closest reading is that reporting an account's usability would over-claim without it.

Widened: the slice 029 review found the same silence over a larger set. `src/services/assets.rs` names ten of the child's user-scope assets and calls each one a fact carried against the launch obligation, while the registry holds no entry for any of them — and here the wrapper does launch through the names, so the obligation is not in doubt, only the registration. The set also shows what registering costs: eight of the ten are ordinary English words, so extending the literal list would demand an entry for every sentence containing `rules` or `commands`, and `dead()` refuses an entry the scan cannot reach, so a naive registration fails the gate instead of passing it.

Exit: ADR, decide whether observing and deleting an artifact is a carry the registry governs at all, or whether the registry is about names the wrapper puts in front of the child — and, for the asset set, whether an entry may declare itself unreachable by the scan rather than being omitted. Registering a scannable name means extending the literal list, which makes the gate enforce every future mention; deciding a class is out of scope means saying so in the registry header, because its current silence reads as coverage.

## Q-013 — Is a session a terminal, or a running agent?

Blocks: whether [ADR-0102](../decisions/ADR-0102-key-child-state-by-terminal.md) and the naming ladder under it stay, and with them the `orphaned` and `unknown` verdicts [ADR-0112](../decisions/ADR-0112-keep-only-the-session-proven-live.md) has to carry.

Raised: slice 035 found that every verdict but `live` traces back to keying a session by its terminal. A terminal is a reusable slot, its name is issued per namespace so a host and a container spell one pane two ways, and it exists whether or not an agent is running in it — which is why every passthrough materialises a directory whether or not it starts an agent, and why one that does leaves the directory standing when the agent exits. The stated intent is narrower than the key: a session is a running coding agent, from a terminal user interface or a command-line execution, and `doctor` on either side of the wrapper is not one. Keyed by the agent instead, liveness is the recorded process under the recorded boot, which is decidable on a host, in a container, and in a virtual machine without a namespace ladder, and no directory outlives what it stands for.

Exit: ADR, decide whether the session key is the terminal or the agent process, then move ADR-0102, the naming ladder, the witness record, and the on-disk layout together. The trade the decision turns on is continuity: keying by terminal is what lets one pane's `.claude.json` and history carry across relaunches, so an agent-keyed layout has to say where that state lives and whether reopening a pane still resumes it. Existing trees are keyed the old way, so the record also owns whether they migrate or are collected.
