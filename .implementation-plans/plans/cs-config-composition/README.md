# claude-session — Config Composition (JSON pieces + YAML manifest → settings.json)

> Complexity: L | Rounds: 4 | Generated: 2026-06-19 | Repo: /workspaces/claude-session

## Problem Statement

`claude-session` is the configuration source of truth; native `claude` config is BLIND to the user. Users edit only `~/.config/claude-session/**` (dotfile-managed/stowed). They compose **partial native `settings.json` JSON pieces** through a **YAML manifest** — the manifest IS the user-facing "profile" — and `claude-session` generates the final native `settings.json` (and manages `.claude.json` trust/state) that the real `claude` reads inside the isolated session. The model follows `devcontainerctl` (JSON pieces ordered by a YAML manifest, last-wins) but is reimplemented idiomatically in Rust (`serde_json`), improving on it with **per-key provenance**, **configurable per-key array strategies**, and **schema validation**. Depends on `cs-foundation` (figment config, error/ui) and `cs-isolation` (the session dir the generated `settings.json` lands in); runs in parallel with `cs-wrapper-runtime`/`cs-accounts-auth`.

## Strategy

Four rounds. R1 builds the XDG config layout and the manifest/piece models with loader validation. R2 builds the `serde_json::Value` deep-merge engine (per-key strategies + provenance). R3 generates the native `settings.json` atomically into the resolved session dir, with schema validation, freshness, a provenance sidecar, and `.claude.json` trust/state handling. R4 exposes the `config`/`profile` verbs.

## Rounds

1. `config-layout-and-models.md` — XDG config tree, manifest/piece models, loader validation.
2. `merge-engine.md` — serde_json deep merge, per-key strategies, per-key provenance.
3. `generate-settings.md` — atomic native settings.json output, schema validation, freshness, provenance sidecar, `.claude.json` trust/state.
4. `config-commands.md` — `config`/`profile` verbs (`show|path|compose|validate|status`, `list|show`).

## Execution Commands

```bash
# Execute the next todo round (executor reads queue-rounds.yaml, runs the first todo round, then stops):
/prex -ar @.implementation-plans/plans/cs-config-composition/

# Or target a specific round file directly:
/prex -ar .implementation-plans/plans/cs-config-composition/config-layout-and-models.md
```

## Execution Discipline

**Rounds must be executed one at a time.** Each round is a self-contained unit of work designed for a single `/prex` session. Do not implement multiple rounds in one session.

When `/prex` is pointed at this directory or this `README.md`, it MUST:

1. Read this plan's `queue-rounds.yaml`.
2. Find the first round with status `todo`.
3. Set that round's `status` to `doing`, execute ONLY that round, then set it to `done` and stop.
4. End the session — a fresh `/prex` session is launched for any subsequent round.

## Decisions & Constraints

- `Executor: prex (EF 1.5)`.
- **JSON pieces + YAML manifest → generated native `settings.json`.** Source pieces are partial `settings.json` JSON files; a YAML manifest IS the profile and declares an ordered `layers: [...]` list (last-wins). NOT TOML source.
- **Manifest schema** (devcontainerctl model): one YAML file per profile, field `layers: [string,...]` (ordered, `minItems:1`, `additionalProperties:false`), validated.
- **Merge rules** (improve on devcontainerctl's jq): idiomatic Rust `serde_json::Value` recursive merge; scalars last-wins; arrays **configurable per-key** (default `replace`; opt-in `concat` / `merge-by-key`); objects merge-by-key. Record **per-key provenance** (which piece set each key) and schema-validate the merged `settings.json`.
- **Atomic output** (tempfile → rename) into the resolved account/session dir; **freshness** check must inspect every referenced piece's mtime (not just the manifest); a provenance sidecar (`.claude-session-compose.json`).
- User config at `~/.config/claude-session/` (manifests + pieces), dotfile-managed; native `claude` config stays blind to the user.

## Rejected Alternatives

- **TOML source pieces** (codex flavor) — rejected by Decision 4; claude natively reads JSON `settings.json`, so pieces are JSON.
- **jq/shell merge** (devcontainerctl's engine) — rejected; reimplement in Rust `serde_json` for type-safety, provenance, and schema validation.
- **No provenance / no schema validation** (devcontainerctl) — rejected; the brief asks to improve on the model.

## Risks & Edge Cases

- Conflicting array-merge intent across keys: default to `replace` (safe last-wins), make concat/merge-by-key opt-in per key; document the default.
- A piece with an unknown/typo'd key: surface it with provenance (which file), aligned with `deny_unknown_fields` philosophy, but allow unknown _native_ settings keys unless a maintained schema exists (native settings schema evolves).
- Stale generated `settings.json` when a piece changes but the manifest mtime does not: check every referenced piece's mtime/hash, not just the manifest's.
- Effort-level / live `/effort` picker divergence (shell-tool weakness): document that composed effort values can be overridden live; do not fight the picker.

## Completion

When all rounds are done, set each round `done` in this plan's `queue-rounds.yaml` and set this plan `done` in the top-level `.implementation-plans/queue-plans.yaml`. Nothing moves on disk.
