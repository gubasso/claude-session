# claude-session — Config Composition (JSON pieces + YAML manifest → settings.json)

> Complexity: L | Rounds: 4 | Generated: 2026-06-19 | Repo: repository root

## Problem Statement

`claude-session` is the configuration source of truth; the user never edits the child's files directly. Users author **partial JSON settings pieces** and an ordered **YAML manifest** — the manifest is the user-facing "profile" — and `claude-session` generates the final settings file, plus the child's trust state, inside the isolated session. The model is base-and-overlay composition, reimplemented in Rust over `serde_json` and extended with **per-key provenance**, **configurable per-key array strategies**, and pragmatic validation. It is specified in `docs/reference/configuration.md` and recorded in `docs/decisions/0010-compose-native-settings-from-declared-layers.md`. Depends on `cs-foundation` (config, error, ui) and `cs-isolation` (the session dir the generated file lands in); runs in parallel with `cs-wrapper-runtime` and `cs-accounts-auth`.

## Strategy

Four rounds. R1 builds the XDG config layout and the manifest/piece models with loader validation. R2 builds the `serde_json::Value` deep-merge engine (per-key strategies + provenance). R3 generates the native `settings.json` atomically into the resolved session dir, with schema validation, freshness, a provenance sidecar, and `.claude.json` trust/state handling. R4 exposes the `config`/`profile` verbs.

## Rounds

1. `config-layout-and-models.md` — XDG config tree, manifest/piece models, loader validation.
2. `merge-engine.md` — serde_json deep merge, per-key strategies, per-key provenance.
3. `generate-settings.md` — atomic native settings.json output, schema validation, freshness, provenance sidecar, `.claude.json` trust/state.
4. `config-commands.md` — `config`/`profile` verbs (`view|path|compose|validate|status`, `list|status`).

## Execution Commands

Any executor following [the contract](../../README.md#the-executor-contract) can run these rounds. `/prex` is the one used to generate them, shown here as a worked example:

```bash
# Execute the next todo round (executor reads queue-rounds.yaml, runs the first todo round, then stops):
/prex -ar @.implementation-plans/plans/cs-config-composition/

# Or target a specific round file directly:
/prex -ar .implementation-plans/plans/cs-config-composition/config-layout-and-models.md
```

## Execution Discipline

Execution follows the executor contract in [`../../README.md`](../../README.md#the-executor-contract), which owns the rule: one round per session, first `todo` round only, status transitions in `queue-rounds.yaml`, stop.

This plan adds no exceptions to it.

## Decisions & Constraints

- **Executor provenance:** `prex (EF 1.5)` — the profile these rounds were generated with. Provenance only; see [the contract](../../README.md#the-executor-contract).
- **JSON pieces + YAML manifest → generated native `settings.json`.** Source pieces are partial `settings.json` JSON files; a YAML manifest IS the profile and declares an ordered `layers: [...]` list (last-wins). NOT TOML source.
- **Manifest schema**: one YAML file per profile whose sole required field is an ordered, non-empty `layers` list. Unknown fields rejected, empty list rejected. Specified in `docs/reference/configuration.md`.
- **Merge rules**: a recursive `serde_json::Value` merge — objects by key, scalars last-wins, arrays **replace by default** with `concat` and `merge-by-key` opt-in **per key**. The merge is deterministic (identical inputs, byte-identical output) or freshness and diffs are both useless. Record **per-key provenance**. Specified in `docs/reference/configuration.md`.
- **Atomic output** (tempfile → rename) into the resolved account/session dir; **freshness** check must inspect every referenced piece's mtime (not just the manifest); a provenance sidecar (`.claude-session-compose.json`).
- User config at `~/.config/claude-session/` (manifests + pieces), dotfile-managed; native `claude` config stays blind to the user.

## Rejected Alternatives

- **TOML source pieces** — rejected; the child reads JSON, so pieces are JSON and no format translation sits in the middle.
- **A shell-and-`jq` merge pipeline** — rejected; implemented in Rust over `serde_json` for type safety, provenance, and validation.
- **Composition without provenance** — rejected; without it, "this setting is wrong" cannot be turned into "this piece overrode that one" short of bisecting files.

## Risks & Edge Cases

- Conflicting array-merge intent across keys: default to `replace` (safe last-wins), make concat/merge-by-key opt-in per key; document the default.
- A piece with an unknown/typo'd key: surface it with provenance (which file), aligned with `deny_unknown_fields` philosophy, but allow unknown _native_ settings keys unless a maintained schema exists (native settings schema evolves).
- Stale generated `settings.json` when a piece changes but the manifest mtime does not: check every referenced piece's mtime/hash, not just the manifest's.
- Effort-level / live `/effort` picker divergence (shell-tool weakness): document that composed effort values can be overridden live; do not fight the picker.

## Completion

When all rounds are done, set each round `done` in this plan's `queue-rounds.yaml` and set this plan `done` in the top-level `.implementation-plans/queue-plans.yaml`. Nothing moves on disk.
