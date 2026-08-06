# 011 — Entry point and contract repair

<!-- markdownlint-configure-file { "MD043": { "headings": ["*","## Goal","## Appetite","## Core","## In scope","## Out of scope","## Governed by","## Acceptance","## Rabbit holes","## Done when","## Revisions"] } } -->

## Goal

Make the entry point a boundary and make the contracts slice 001 claimed actually hold.

## Appetite

3 implementation sessions.

## Core

The failure a user hits is rendered, logged, and classified in that order, and `--` stays unconditional.

## In scope

- The entry point split into a boundary and a fallible program, ordered report, flush, then exit.
- The `--` sentinel, `--quiet`, the default diagnostic mirror, swallowed spawn failures, configuration error typing, and the project layer ceiling.
- Adapter ports as traits, so a service takes its dependencies as parameters and a fake can stand in.
- One boundary error carrying a kind and a diagnostic, replacing the variant list repeated across six matches.
- Removal of every binding that exists only to silence a dead-code lint.

## Out of scope

- Signal supervision, which slice 003 owns.
- Private-mode and symlink hardening of the filesystem adapter, which slice 002 owns.
- Account and profile semantics, which slices 004 and 005 own.
- Removing a dependency, which follows its own admission procedure.
- The colour ladder, which needs a decision rather than a repair.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Architecture](../../../explanation/architecture.md)
- [Wrapper model](../../../explanation/wrapper-model.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Configuration](../../../reference/configuration.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [ADR-0002](../../../decisions/ADR-0002-verbatim-argv-passthrough.md)
- [ADR-0005](../../../decisions/ADR-0005-exit-code-taxonomy.md)
- [ADR-0035](../../../decisions/ADR-0035-convert-the-typed-error-to-a-code-once.md)

## Acceptance

- When a wrapper verb spelling follows the sentinel, the wrapper shall forward it to the child unchanged. -> passthrough::a_wrapper_verb_behind_the_sentinel_reaches_the_child
- When a wrapper operation fails, the wrapper shall write its error record to the log before the sink is flushed. -> output::a_failing_invocation_records_its_error_in_the_log
- When the child is terminated by a signal, the wrapper shall reproduce that death after the log sink is flushed. -> passthrough::a_signalled_run_still_leaves_a_complete_log
- Where `--quiet` is supplied, the wrapper shall emit no diagnostic below the error level. -> output::the_verbosity_ladder_governs_the_diagnostic_mirror
- If a file the user named cannot be read, then the wrapper shall report the typed condition rather than a generic configuration failure. -> configuration::an_unreadable_named_config_is_typed
- While no repository boundary is found above the working directory, the wrapper shall apply no project layer. -> configuration::no_project_layer_applies_outside_a_repository
- When a subroutine child cannot be spawned, the wrapper shall name that condition in place of the composed section. -> output::a_spawn_failure_names_its_condition_in_place_of_the_section
- The wrapper shall resolve its child through an injected port, so a service test needs no process. -> services::child::tests::a_refused_candidate_does_not_end_the_walk

## Rabbit holes

- Genericising `AppContext` over its ports; escape: keep the context concrete and put the trait parameters on the service signatures.
- Rewriting the argument split to carry the sentinel; escape: add one field to the existing partition and gate the verb rescue on it.
- Building the signal matrix while the entry point is open; escape: slice 003 owns it, and the entry point only reproduces what the wait reports.
- Implementing the colour ladder to justify the code that computes it; escape: record the question and leave both alone.

## Done when

The entry point holds no logic, every acceptance line resolves to a test that fails without its fix, and `just hooks` is green.

## Revisions

- 2026-08-06: `which` left the manifest. Unifying the two `PATH` walks met the retirement condition the dependency reference already named for it, so the crate had no remaining call site.
- 2026-08-06: The default diagnostic mirror is warnings and above in human output and off in JSON, rather than off everywhere. The verbosity table and the sentence calling the mirror opt-in disagreed; the JSON error document owns standard error, which decides it.
- 2026-08-06: A filesystem fault during the `PATH` walk now outlives the walk instead of being swallowed into a not-found. Reporting a failed access check as a denial told the user to repair a mode that was already correct.
