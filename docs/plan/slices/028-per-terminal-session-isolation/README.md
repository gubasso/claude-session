# 028 — Per-terminal session isolation

## Goal

Give each terminal its own child state directory while one account keeps one saved login and one project history, so two panes of one account stop overwriting the three files the child keys by nothing.

## Appetite

3 implementation sessions.

## Core

Two terminals of one account write separate `.claude.json` and `history.jsonl`, share one `.credentials.json`, and share one `projects/` tree.

## In scope

- One decision superseding [ADR-0065](../../../decisions/ADR-0065-retire-the-terminal-group.md), reinstating a terminal axis for the child's own state directory alone and leaving [ADR-0064](../../../decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md) untouched, since composed settings stay keyed by profile and input digest.
- A terminal derivation ladder, reviving the reasoning recorded in the superseded [ADR-0062](../../../decisions/ADR-0062-derive-the-group-from-the-controlling-terminal.md) without its retired `--session` and environment rungs: the controlling terminal, then the session leader, then a refusal.
- One decision permitting a symbolic link at a wrapper-owned name whose target the wrapper verifies, amending the no-symlink rule of [ADR-0061](../../../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md) and the security table in [XDG storage](../../../reference/xdg-storage.md), because a shared `projects/` tree is unreachable without one.
- One decision carrying the child's credential-store variable against the launch obligation of [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md), so per-terminal configuration directories share the one credential [ADR-0025](../../../decisions/ADR-0025-share-one-native-login-per-account.md) requires.
- An amendment to [ADR-0098](../../../decisions/ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md), moving the seed from login to launch because no session directory exists at login, and adding the workspace-trust key behind a configuration gate.
- A configuration key for that trust seed, through the layering and provenance [configuration](../../../reference/configuration.md) owns, with the generated examples regenerated.
- Two catalog checks under [ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md), reporting the derived terminal and the declared links.
- Registration of the carried variable in [child facts](../../../reference/child-facts.yaml) and its freshness in [research tracking](../../../reference/research-tracking.yaml), since the child documents it nowhere.

## Out of scope

- Pruning stale session directories, which no complaint has yet asked for and which would add an unattended destructive path to every launch.
- Supplying the child's user-level assets, which is 029 and needs the link mechanism this slice builds.
- Any change to argv, stream, or status passthrough, or to the exec of [ADR-0084](../../../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md).
- Keying composed settings by anything but profile and input digest.
- Resolving [Q-012](../../open-questions.md), whose subject is whether a carried filename belongs in the registry at all.
- Measuring the child's cross-process refresh behaviour, which is [Q-002](../../open-questions.md) and is not what this slice changes.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md)
- [ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)
- [ADR-0025](../../../decisions/ADR-0025-share-one-native-login-per-account.md)
- [ADR-0061](../../../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md)
- [ADR-0064](../../../decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md)
- [ADR-0065](../../../decisions/ADR-0065-retire-the-terminal-group.md)
- [ADR-0084](../../../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0098](../../../decisions/ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Configuration](../../../reference/configuration.md)
- [Accounts](../../../reference/accounts.md)
- [Process runtime](../../../reference/process-runtime.md)
- [Doctor](../../../reference/doctor.md)
- [Child facts](../../../reference/child-facts.yaml)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When the wrapper launches the child, it shall point the child's configuration directory at a directory derived from the controlling terminal.
- When the wrapper launches the child, it shall point the child's credential store at the account directory that holds the saved login. -> accounts::a_launch_splits_state_by_terminal_and_shares_the_login_and_projects
- When two terminals of one account launch, each shall write its own child configuration file and its own prompt history.
- When two terminals of one account launch, both shall read one saved login and write one project tree.
- If no controlling terminal and no session leader resolve, then the wrapper shall refuse before the exec and name what it could not derive.
- Where a wrapper-managed component is a symbolic link the wrapper did not declare, the wrapper shall refuse as it does today. -> services::storage::guard::tests::a_link_above_the_declared_name_is_still_refused
- Where a declared link points somewhere other than the target the wrapper recorded, the wrapper shall refuse. -> services::storage::guard::tests::a_declared_link_is_accepted_only_where_and_where_it_points
- When a launch materialises a session directory, it shall seed the one child key that directory's newness makes the child ask.
- While the trust key is enabled, a launch shall record the working directory as trusted in the session directory it created. -> accounts::a_launch_records_the_working_directory_as_trusted
- When the work lands, no current record shall assert that no terminal input remains in the storage layout.

## Rabbit holes

- Reinstating the retired group with it — the composed-settings key, the claim protocol, the derivation fingerprint, `--session`, and the environment rung; escape: this slice derives one path component and nothing else reads it.
- Generalising the declared link into a link subsystem with arbitrary targets; escape: the wrapper declares each link at a fixed name with a fixed target and verifies exactly that.
- Copying the credential into each session directory when the store variable looks awkward; escape: copying is the failure [ADR-0025](../../../decisions/ADR-0025-share-one-native-login-per-account.md) exists to prevent, and one file is the whole point.
- Splitting `projects/` because the rest of the directory splits; escape: memory lives under it, and a per-terminal memory is the defect observed in the predecessor.
- Deciding the registry question for every carried filename while registering one variable; escape: [Q-012](../../open-questions.md) owns that, and this slice registers what the scan can see.

## Done when

Two terminals of one account keep separate child configuration and history over one login and one project tree, the derivation refuses rather than inventing a directory, every record the change contradicts has moved, and `just hooks` is green.

## Revisions

None.
