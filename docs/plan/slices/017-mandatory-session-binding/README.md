# 017 — Mandatory session binding

## Goal

Make every launch attributable: no run reaches the child without a resolved account and a resolved profile, so the answer to "which credential and which settings am I under" is never "none".

## Appetite

2 implementation sessions.

## Core

A launch with no resolved account or no resolved profile does not exec the child.

## In scope

- One decision superseding [ADR-0058](../../../decisions/ADR-0058-behave-as-stock-claude-by-default.md), replacing the stock-`claude` default with a bound-launch default and naming the divergence it buys.
- A binding gate on the launch path, refusing before the exec, with an error naming which of the two selections is missing and a hint carrying the next action.
- One decision fixing what a tree with no configuration binds to, recording the rejected options rather than shipping them: a refusal that points at the shipped examples, or a wrapper-materialized starting pair, which reverses [ADR-0015](../../../decisions/ADR-0015-retire-the-init-verb.md) and the no-write rule of [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md).
- Alignment of [configuration](../../../reference/configuration.md) where it states that an empty tree launches the child unchanged, and of its selection ladders where the terminal rung is nothing.
- Alignment of [accounts](../../../reference/accounts.md) where rung four is nothing and a passthrough may carry neither wrapper authentication variable.
- Alignment of the [charter](../../charter.md) purpose line, which states the opposite default.
- A profile rung for the last-used marker, or a recorded rejection of one, since only the account ladder is sticky today and the profile is resolved fresh on every invocation.
- The exit code for an unbound launch, taken from the existing taxonomy in [exit codes](../../../reference/exit-codes.md).

## Out of scope

- Any change to argv, stream, or status passthrough for a launch that does bind; [ADR-0002](../../../decisions/ADR-0002-verbatim-argv-passthrough.md) and [ADR-0005](../../../decisions/ADR-0005-exit-code-taxonomy.md) are untouched.
- Letting a project file supply the account, which [ADR-0071](../../../decisions/ADR-0071-restrict-the-project-layer-to-the-profile-key.md) refuses on its own grounds.
- Announcing the bound pair on every launch, which is a presentation surface and a separate need.
- The wrapper's own verbs, which take no binding because none of them is a passthrough launch; the ones that spawn the child do so as a subroutine of the verb rather than as the session the wrapper hands over to.
- The friction entries the usability pass raised against help output and the installed spelling.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0058](../../../decisions/ADR-0058-behave-as-stock-claude-by-default.md)
- [ADR-0015](../../../decisions/ADR-0015-retire-the-init-verb.md)
- [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md)
- [ADR-0071](../../../decisions/ADR-0071-restrict-the-project-layer-to-the-profile-key.md)
- [ADR-0084](../../../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md)
- [ADR-0090](../../../decisions/ADR-0090-require-account-and-profile-before-child-launch.md)
- [ADR-0091](../../../decisions/ADR-0091-refuse-an-unconfigured-first-launch-without-scaffolding.md)
- [Configuration](../../../reference/configuration.md)
- [Accounts](../../../reference/accounts.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Presentation](../../../reference/presentation.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When a launch resolves no account, the wrapper shall refuse before the exec and name the missing selection. -> accounts::unselected_passthrough_never_runs_the_version_probe
- When a launch resolves no profile, the wrapper shall refuse before the exec and name the missing selection. -> session_storage::an_account_selected_profile_missing_launch_is_refused
- When a launch resolves both, the child shall receive the same argv, streams, and status it receives today.
- When a bare invocation resolves `default_account` and `default_profile` from user configuration, the child shall launch with that bound session. -> session_storage::a_bare_invocation_uses_configured_account_and_profile_defaults
- Where a document states that an empty configuration tree launches the child unchanged, it shall state the binding requirement instead.
- When the decisions land, no current record shall assert both the stock-`claude` default and the bound-launch default.

## Rabbit holes

- Reopening what a profile composes or how pieces merge; escape: this slice decides whether a profile is required, never what one contains.
- Building a scaffolding subsystem behind the fresh-tree decision; escape: the decision picks one starting behaviour and records the other, and a scaffold that grows beyond that is its own slice.
- Treating the binding as a passthrough break and re-arguing [ADR-0002](../../../decisions/ADR-0002-verbatim-argv-passthrough.md); escape: refusing to launch changes no argv, and the record that moves is the transparency default.

## Done when

Both decisions are recorded and applied, an unbound launch never reaches the exec, every owner naming the old default has moved with it, and `just hooks` is green.

## Revisions

- 2026-08-12 — Learned that launch readiness is resolution-based, so a bare invocation with both user defaults must reach the child; added matching acceptance evidence.
