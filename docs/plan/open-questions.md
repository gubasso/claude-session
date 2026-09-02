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

Widened again: slice 037 carries the child's own name for a session so the report about it names its subject as its reader does, under the fourth obligation [ADR-0114](../decisions/ADR-0114-name-a-reported-session-as-the-child-does.md) adds. The registry entry is filed under `procStart`, the half of the record the scan can see, while `name` — the half the carry exists for — is an ordinary English word it cannot, so the entry justifies its companion in prose and the gate enforces only the joinder. That is the same limit under a new obligation rather than a new limit, and it is the second class of carry the scan reaches only sideways.

Exit: ADR, decide whether observing and deleting an artifact is a carry the registry governs at all, or whether the registry is about names the wrapper puts in front of the child — and, for the asset set, whether an entry may declare itself unreachable by the scan rather than being omitted. Registering a scannable name means extending the literal list, which makes the gate enforce every future mention; deciding a class is out of scope means saying so in the registry header, because its current silence reads as coverage.

## Q-014 — Which launch-path guards survive a directory that is always fresh?

Blocks: whether the occupied-name branches of [the asset supply](../../src/services/assets.rs), [the registry adoption](../../src/services/session.rs), and the read-modify-write in [the launch seed](../../src/services/account/onboarding.rs) stay in the tree.

Raised: slice 036 keyed a session to its agent, so a launch creates its session directory and is the only thing that ever writes into it. Every guard that asks "is something already at this name" therefore has no reachable case: an undeclared occupant at an asset seat, a real directory where the peer-registry link goes, a stale link from an earlier boot, and a child configuration written before this launch. Three closed slices lost acceptance lines to this, and the branches now carry no test because no fixture can construct the state they refuse. They are cheap — one `stat` each — and they are the kind of check that is wrong to remove on a reachability argument alone if the key ever changes again.

Exit: ADR, decide whether an unreachable guard is kept as a stated invariant or removed under [ADR-0051](../decisions/ADR-0051-let-every-surface-element-discriminate.md), which asks every surface element to discriminate through a current use. Removing them also settles what [ADR-0098](../decisions/ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md) and [ADR-0105](../decisions/ADR-0105-seed-a-session-at-launch.md) may promise about preserving keys beside the one a launch seeds, since there are none left to preserve.

## Q-015 — What collects a prior boot's sessions where the namespace is boot-derived?

Blocks: whether [sessions](../reference/sessions.md) should state that its ended-boot row is unreachable wherever the namespace component is boot-derived, and whether the launch sweep should reach those directories at all.

Raised: the slice 036 review observed that [the judgment](../../src/domain/witness.rs) asks whether the record's namespace is this run's before it asks which boot the record names, so a namespace mismatch answers first. Where [ADR-0109](../decisions/ADR-0109-discriminate-namespaces-across-kernels.md)'s ladder fell to the boot identifier, a reboot changes the namespace component too, and the previous boot's directories are `foreign` rather than `foreign-boot` — `unknown` instead of `dead`, which the launch sweep leaves and only `session clean` removes. The order cannot simply be swapped: another kernel sharing this tree also names a different boot, and asking about the boot first would collect a directory whose agent is running on that other machine. Under ADR-0102 the cost was one stranded directory per terminal; under [ADR-0113](../decisions/ADR-0113-key-a-session-to-its-running-agent.md) it is one per launch, so what a recorded posture absorbed is now unbounded growth between explicit collections.

Exit: ADR, decide whether the durable record carries enough identity to tell this machine's ended boot from another kernel's live one — which is what the namespace component alone cannot do — or whether the boot-derived rung keeps stranding and the reference page says so plainly instead of promising the sweep. Either answer also settles whether `session clean` stays the only thing that reaches these directories.

## Q-016 — Why does `config` report an unreadable file as one it never looked for?

Blocks: whether [configuration](../reference/configuration.md#schema) may keep claiming that an unknown key "is rejected, not ignored" and that "the rejection names the offending key, its file, and, where the distance is small, the key it was probably meant to be".

Raised: slice 038 added a key and measured what an older wrapper does when it meets one it does not know. `doctor` is exact — `wrapper-config-parses` fails and names the field and every key it expected. The `config` verb is not: its Files consulted section says "No configuration file was looked for", and its Problems section is empty, for a file that was looked for, found, read, and refused. A reader who runs the verb whose job is to explain the configuration is told the opposite of what happened, and the one whose job is to check it is told the truth.

The claim in the reference is therefore true of the launch path and of `doctor`, and false of the verb a person reaches for first. Nothing here is a launch defect: a refused file leaves the account unbound and the launch refuses under [ADR-0090](../decisions/ADR-0090-require-account-and-profile-before-child-launch.md).

Exit: ADR, decide whether `config` grows a Problems row carrying the decode error, or whether Files consulted distinguishes "absent" from "refused" and the existing empty section is enough. Either changes a published report shape, so it needs a slice rather than an edit.

## Q-017 — Why do the cargo-backed hooks fail intermittently?

Blocks: whether `just hooks` can be read as a verdict, since a green run and a red one are the same inputs, and whether a red run should be retried or investigated.

Raised: slice 038's review measured `scripts/check-acceptance-tests` failing once in five identical runs, each time naming a different unresolved ID — `cli_artifacts::man_derives_the_root_page_from_the_parser_tree`, then `sessions_gc::every_verdict_reports_the_ground_it_stands_on`, then `sessions_gc::a_launch_collects_the_sessions_whose_agents_exited`. None of those tests, nor the slices naming them, were touched by that work. Both halves of the comparison look deterministic in isolation: `cargo nextest list --all-features` was byte-identical across four samples at 608 lines, and the named set held at 149 every run. The failures cluster on the first run after a rebuild, but a listing captured immediately after a forced rebuild was also byte-identical, so that correlation is not yet a cause. The script already guards the failure mode nearest this one, refusing a listing whose command exited non-zero, and that guard did not fire.

Widened: the same slice later saw the `cargo nextest (integration tests)` hook fail inside `just hooks` while the identical lane passed three times out of three when run on its own, immediately afterwards and against an unchanged tree. That is a second hook, sharing only the cargo build directory with the first, so the cause is more likely contention between the cargo-backed hooks in one `just hooks` run than anything in the resolver's own comparison. A rerun has cleared every occurrence so far, which is what makes this a question rather than a defect report.

Exit: measurement, run `just hooks` in a loop capturing each hook's full output, and on a failing iteration keep the resolver's listing file, its named set, and the nextest output together. Until a failing run's inputs are in hand there is nothing to decide; the fix follows from whether the two hooks fail for one reason or two.

## Q-018 — Does a name the wrapper only tells a person to type belong in the registry?

Blocks: whether `CLAUDE_CODE_PLUGIN_CACHE_DIR` keeps its `contested` entry in [child facts](../reference/child-facts.yaml), and whether [ADR-0089](../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md) grows a fourth obligation or rules the carry out.

Raised: slice 038's review found that the plugin guide's build recipe was wrong — it moved a finished seed, which is the one thing that guide warns makes a seed load nothing. The correct route is the child's own `CLAUDE_CODE_PLUGIN_CACHE_DIR`, which builds the tree at its final path and needs no rewrite. Naming it in the guide is what makes the recipe correct, and it also makes the name a carry the discovery scan reaches, because it wears a scanned prefix.

None of ADR-0089's obligations fits. The wrapper does not launch through it, no wrapper spelling collides with it, and the wrapper neither sets nor reads it, so it cannot be over-claiming an effect. What it is, is a name the wrapper must be able to say out loud in a procedure the user performs, because the wrapper writes no user configuration ([ADR-0015](../decisions/ADR-0015-retire-the-init-verb.md)) and therefore cannot build the tree it reads. The three obligations were written for names the wrapper itself uses at run time, and this one is used by a person following a document.

The alternative is to drop the name and describe the variable without spelling it, which makes the recipe unusable, or to publish the fragile move-and-rewrite route instead, which the same review rejected. Neither is better than an entry with an honest label.

Exit: ADR, decide whether ADR-0089 grows an obligation covering a name a guide must spell, or whether the registry's scope narrows to run-time carries and documentation is enforced at review. Either way the `contested` entry leaves with the decision.
