# Docs & Hardening R1: Prior-Art Research & headroom Integration Docs

> Plan: cs-docs-hardening | Round: 1 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: /workspaces/claude-session

## Context

The project brief requires the web prior-art research to be captured, classified, and saved into the repo docs as INSPIRATION, and the headroom integration to be documented. This round writes the competitive-analysis/prior-art note and the headroom integration guide into the Diátaxis zones established by `cs-foundation`. The headroom seam (`ANTHROPIC_BASE_URL` injection) was built in `cs-wrapper-runtime`; this round documents how to use it.

## Previous Rounds

`cs-foundation`: `docs/` Diátaxis skeleton (`decisions/`, `guides/`, `reference/`, `explanation/`). `cs-wrapper-runtime`: the injectable `ANTHROPIC_BASE_URL` child-env seam. Expect both to exist.

## Scope of This Round

- IN scope: a prior-art / competitive-analysis note in `docs/explanation/` (or `docs/reference/`) classifying the landscape — `aisw` (closest Rust prior art), the cc-account-switchers, `claude-swap` (session mode), `claude-wrapper`, `claude-code-env`, `kustomize` (config layering inspiration), `bubblewrap`/`firejail` (future hardened isolation), and the native `CLAUDE_CONFIG_DIR`/`.claude.json` /`ANTHROPIC_*` facts — with sources cited and a short "what we borrow as inspiration" per tool; a headroom integration guide in `docs/guides/` documenting `headroom proxy --port` + `ANTHROPIC_BASE_URL=http://localhost:<port>` injected via claude-session's child-env seam, with the exact commands and the "no internal compression" boundary.
- OUT of scope: doctor hardening (round 2), completions/man pages (round 3), ADR closeout/release (round 4).

## Current State

### Key Files

- `/workspaces/claude-session/docs/explanation/` — add the prior-art note (zone for understanding).
- `/workspaces/claude-session/docs/guides/` — add the headroom integration guide (task zone).
- `/workspaces/claude-session/docs/README.md` — index already exists; ensure new docs are linked.

### Existing Patterns

Source material (brief §9–§10, INSPIRATION only — cite sources): headroom is a token-compression proxy; the clean seam is `ANTHROPIC_BASE_URL` → `headroom proxy`. Prior art: `aisw` (Rust, native-env-var per profile dir), cc-account-switcher (encrypted creds), claude-swap (session mode + quota auto-pick), kustomize (base+overlay), bubblewrap/firejail (namespace sandbox). Native facts: `CLAUDE_CONFIG_DIR` (undocumented, leaky, comma-separated), `.claude.json`, settings precedence, `ANTHROPIC_*`. Diátaxis: zone-first; explanation = understanding, guides = task. ARID: write each fact once, link.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: prior-art-and-headroom-docs`) `status` to `doing`.

### Step 1: Prior-art note

Write `docs/explanation/prior-art.md` (or `reference/`) classifying the landscape with a table (name, URL, language, what it does, overlap, what we borrow) and the native-`claude` config/session facts, with sources cited.

### Step 2: headroom guide

Write `docs/guides/headroom-integration.md`: how to run `headroom proxy` and inject `ANTHROPIC_BASE_URL` via claude-session, with exact commands and the "no internal compression" boundary.

### Step 3: Link + lint

Link both from `docs/README.md`; ensure markdownlint/lychee (if wired) pass.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: prior-art-and-headroom-docs`) `status` to `done`.

## Acceptance Criteria

- [ ] A prior-art/competitive-analysis note exists in a Diátaxis zone, classifying the landscape with sources cited and a per-tool "borrow as inspiration" line.
- [ ] A headroom integration guide documents the `ANTHROPIC_BASE_URL` → `headroom proxy` seam with exact commands and the no-internal-compression boundary.
- [ ] Both are linked from `docs/README.md`; doc lints pass.
- [ ] This plan's `queue-rounds.yaml` shows round `prior-art-and-headroom-docs` as `done`.

## Next Round

Round 2 (`doctor-hardening`) turns `doctor` into a comprehensive, graceful health check across all subsystems.
