# 024 — Launch ready after login

## Goal

Make one `account login` enough. Today a login reports success, every account surface calls the account healthy, and the first bare launch lands in the child's first-run onboarding instead of the prompt — which in token mode is a browser sign-in the account does not need and cannot be completed into the account's stored mode.

## Appetite

2 implementation sessions.

## Core

After one successful `account login`, a bare launch under that account reaches the child's prompt, and an account that would not is said so before the exec rather than discovered inside the child.

## In scope

- One decision to write the single child-owned key a wrapper-built launch cannot otherwise reach, amending [ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md) against the launch obligation rather than widening it.
- A per-account readiness probe and a one-key writer, under the account's existing lock scope and the atomic write sequence.
- The writer called from both login branches, after the credential and the binding commit, refusing in the shape a post-commit failure already uses.
- One warning before the exec when the selected account would still onboard, naming the account and the verb that repairs it.
- One account-scope `doctor` check carrying a title, a consequence, and a next action, so an account created before this slice is visibly repairable.
- The login report saying that a launch under the account reaches the child's prompt.
- Registration of the carried key in [child facts](../../../reference/child-facts.yaml), which needs the registry's discovery widened past environment-variable spellings before a key of this shape can be registered at all.
- Alignment of [accounts](../../../reference/accounts.md), [configuration](../../../reference/configuration.md), whose child-owned account state paragraph this slice makes false, [doctor](../../../reference/doctor.md), and [session isolation](../../../explanation/session-isolation.md).

## Out of scope

- Seeding any other child-owned state. Theme, trust, project history, and everything else stay the child's, and the workspace trust prompt keeps firing.
- Writing child state on a launch. The guarantee is established once, where no child of that account is running.
- Reading, copying, or repairing the child's saved login, which [ADR-0025](../../../decisions/ADR-0025-share-one-native-login-per-account.md) leaves to the child.
- A repair verb. Re-running `account login` is idempotent and already the documented recovery.
- Modelling the child's onboarding steps or their order, which is the child's and changes without notice.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0011](../../../decisions/ADR-0011-isolate-credentials-by-seed-and-session.md)
- [ADR-0025](../../../decisions/ADR-0025-share-one-native-login-per-account.md)
- [ADR-0026](../../../decisions/ADR-0026-store-and-inject-a-long-lived-subscription-token.md)
- [ADR-0060](../../../decisions/ADR-0060-lock-the-writes-that-are-not-derivable.md)
- [ADR-0080](../../../decisions/ADR-0080-order-the-boundary-as-report-flush-exit.md)
- [ADR-0084](../../../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md)
- [ADR-0087](../../../decisions/ADR-0087-keep-the-credential-lock-beside-the-account.md)
- [ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0093](../../../decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md)
- [ADR-0094](../../../decisions/ADR-0094-give-every-check-a-title-and-a-next-action.md)
- [Accounts](../../../reference/accounts.md)
- [Child facts](../../../reference/child-facts.yaml)
- [Configuration](../../../reference/configuration.md)
- [Doctor](../../../reference/doctor.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Process runtime](../../../reference/process-runtime.md)
- [Presentation](../../../reference/presentation.md)
- [Session isolation](../../../explanation/session-isolation.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When a token login succeeds, the wrapper shall leave the account's child configuration recording that the child's first-run onboarding is complete. -> accounts::a_token_login_records_the_first_run_setup_as_done
- When a native login succeeds, the wrapper shall leave the same record, and every key the child wrote beside it shall survive unchanged. -> accounts::a_native_login_records_it_without_disturbing_the_other_keys
- When the account's child configuration is present and is not an object, the wrapper shall refuse as `DataFormat` and shall leave the stored credential in place. -> accounts::a_child_configuration_that_is_not_an_object_is_refused_and_keeps_the_credential
- When a launch resolves an account whose child configuration would still trigger onboarding, the wrapper shall warn before the exec, name the verb that repairs it, and launch anyway. -> accounts::a_launch_under_an_account_that_would_onboard_warns_and_still_execs
- When `doctor` runs against a selected account that would still onboard, the report shall carry that as a defect with its next action. -> doctor::an_account_that_would_onboard_is_a_defect_with_its_next_action
- When `doctor` runs with no account selected, the readiness check shall be skipped rather than reported as a defect. -> doctor::doctor_skips_inapplicable_session_checks_with_reasons
- When a login is reported, the human form shall say that a launch under the account reaches the child's prompt. -> accounts::a_login_report_says_the_launch_reaches_the_prompt

## Rabbit holes

- Seeding a second key because the child asks another question; escape: one key, and any further question is the child's to ask.
- Rewriting the whole child configuration from a wrapper-side template; escape: read, set one key, write, and refuse anything that is not an object.
- Ensuring readiness on every launch so the guarantee cannot lapse; escape: a launch reads and warns, and only a login writes.
- Modelling the child's onboarding as a state machine to skip it precisely; escape: the single documented key is the whole carry, registered against the launch obligation.
- Widening the child-facts scanner into a general identifier search; escape: an explicit literal list beside the existing prefixes, and nothing else.

## Done when

One `account login` produces an account a bare launch reaches the prompt with, an account that predates the change is named by `doctor` and by a pre-exec warning with the verb that repairs it, the carried key is registered against its obligation, the reference pages that claimed the wrapper seeds nothing say what it seeds and why, and `just hooks` is green.

## Revisions

- 2026-08-13: the login report says this in the human form only. The machine document would have carried a field that is `true` on every login the report is reached from, because the login refuses rather than reports when the record was not written, and [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md) refuses a surface element that discriminates nothing.
