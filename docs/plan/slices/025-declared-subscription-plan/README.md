# 025 — Declared subscription plan

## Goal

Make a token-mode launch arrive as the subscription it is. An injected token carries no plan, so the child reports the session as `Claude API`, withholds the subscription view, and falls back to its non-subscription default model — for a Max account, Sonnet where Opus was expected. The credential is a subscription credential either way; only the wrapper is positioned to say which plan it belongs to.

## Appetite

2 implementation sessions.

## Core

A token login asks which plan the token belongs to, records the answer, and injects it, so the next bare launch reads the account's own plan rather than a fallback; an account that declared none says so before the exec.

## In scope

- One decision to carry the child's subscription-type variable against the launch obligation, because the child reaches its own answer only from state an injected token bypasses.
- The plan as a validated environment value on the account's authentication metadata, written by the same rotation that writes the token, so it cannot outlive the credential it describes.
- A `--plan` option on `account login --token` for a non-interactive login, and a terminal prompt offering the plans the child recognizes plus free entry when the option is absent.
- Injection at exec in token mode only, beside the token the same account already injects.
- One warning before the exec, one account-scope `doctor` check, one `account status` warning, and one login report row, so an account that declared no plan is visibly repairable.
- Registration of the carried variable in [child facts](../../../reference/child-facts.yaml), and of its perishability in [research tracking](../../../reference/research-tracking.yaml), because the variable is undocumented upstream.
- Alignment of [accounts](../../../reference/accounts.md), [doctor](../../../reference/doctor.md), and [the CLI surface](../../../reference/cli-surface.md).

## Out of scope

- Verifying the declared plan. The wrapper does not speak an OAuth endpoint ([ADR-0026](../../../decisions/ADR-0026-store-and-inject-a-long-lived-subscription-token.md)), and the child exposes the plan through no surface a wrapper can read.
- Login mode. The child's own saved credential carries its plan there, so nothing is missing and nothing is injected.
- The child's rate-limit tier, its model selection, and its subscription view. Each is the child's, and none is required to make the plan correct.
- Enumerating the plans as a closed wrapper vocabulary. The prompt offers what the child recognizes today; the stored value is whatever the user declared.
- Refusing a launch, or a non-interactive login, over an undeclared plan. It is reported, never enforced.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0011](../../../decisions/ADR-0011-isolate-credentials-by-seed-and-session.md)
- [ADR-0026](../../../decisions/ADR-0026-store-and-inject-a-long-lived-subscription-token.md)
- [ADR-0027](../../../decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0057](../../../decisions/ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md)
- [ADR-0060](../../../decisions/ADR-0060-lock-the-writes-that-are-not-derivable.md)
- [ADR-0067](../../../decisions/ADR-0067-commit-a-token-rotation-with-the-metadata-rename.md)
- [ADR-0084](../../../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md)
- [ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0093](../../../decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md)
- [ADR-0094](../../../decisions/ADR-0094-give-every-check-a-title-and-a-next-action.md)
- [ADR-0098](../../../decisions/ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md)
- [Accounts](../../../reference/accounts.md)
- [Child facts](../../../reference/child-facts.yaml)
- [CLI surface](../../../reference/cli-surface.md)
- [Doctor](../../../reference/doctor.md)
- [Research tracking](../../../reference/research-tracking.yaml)
- [XDG storage](../../../reference/xdg-storage.md)
- [Process runtime](../../../reference/process-runtime.md)
- [Presentation](../../../reference/presentation.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When `account login --token --plan <name>` succeeds, the wrapper shall record that plan in the account's authentication metadata. -> accounts::a_token_login_records_the_declared_plan
- When a token login runs against a terminal without `--plan`, the wrapper shall ask which plan the token belongs to and record the answer. -> accounts::a_token_login_asks_for_the_plan_and_records_the_answer
- When that prompt is answered with an empty line, the wrapper shall complete the login with no plan recorded rather than refuse. -> accounts::a_declined_plan_prompt_still_completes_the_login
- When `--plan` carries a value outside the declaration's shape, the wrapper shall refuse before ingesting a credential. -> accounts::a_malformed_plan_is_refused_before_ingest
- When a token-mode launch resolves an account with a recorded plan, the child shall receive it in the child's own subscription-type variable. -> accounts::a_token_launch_injects_the_recorded_plan
- When a token-mode launch resolves an account with no recorded plan, the wrapper shall set no such variable, shall warn before the exec, and shall launch anyway. -> accounts::a_launch_without_a_declared_plan_warns_and_still_execs
- When `doctor` runs against a selected token account with no recorded plan, the report shall carry that as a defect with its next action. -> doctor::an_account_without_a_declared_plan_is_a_defect_with_its_next_action
- When `doctor` runs against a selected login-mode account, the plan check shall be skipped rather than reported as a defect. -> doctor::the_plan_check_skips_a_login_mode_account
- When a token login that recorded a plan is reported, the human form shall name the plan the child will read. -> accounts::a_login_report_names_the_declared_plan

## Rabbit holes

- Asking the server which plan the token belongs to; escape: the wrapper does not speak an endpoint, and the declaration is the user's.
- Modelling the plans as a closed enumeration the wrapper validates against; escape: shape validation only, and the four the child recognizes live in the prompt's offer and the registry's rationale.
- Injecting the rate-limit tier beside the plan because they appear together in the child; escape: one variable, against one obligation, and the other earns its own case or none.
- Prompting during a non-interactive login so no account can be created without a plan; escape: `--stdin` never prompts, and an undeclared plan is reported rather than enforced.
- Reconciling a declared plan with the child's own view of the subscription; escape: the child exposes no such view to a wrapper, and a declaration is labelled a declaration on every surface.

## Done when

A token login records the plan it asked for or was given, a bare launch under that account reads it, an account that declared none is named by `doctor`, by `account status`, and by a pre-exec warning with the verb that repairs it, the carried variable is registered against its obligation and tracked as perishable, and `just hooks` is green.

## Revisions

- None.
