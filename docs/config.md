# Configuration

`claude-session` draws its runtime settings from three sources:

1. **CLI flags** (for example `--profile vertex`, `--config /path/to/config.env`)
2. **Environment variables** (`CLAUDE_SESSION_*`)
3. **Config file**: `$XDG_CONFIG_HOME/claude-session/config.env`

Precedence is flags > env > file.

## File locations

| Path | Purpose |
|------|---------|
| `$XDG_CONFIG_HOME/claude-session/config.env` | Global wrapper config only. Dotenv `KEY=VALUE` lines. No profile-specific settings live here. |
| `$XDG_CONFIG_HOME/claude-session/profiles/<name>.yaml` | Profile manifest. YAML with one top-level `settings-layers` array. |
| `$XDG_CONFIG_HOME/claude-session/settings/<name>.json` | JSON settings layer file. Layer names come from manifest entries. |
| `<session_dir>/settings.json` | Session-local composed Claude settings file, written only in manifest mode. |
| `<session_dir>/.claude-session-compose.json` | Session-local sidecar with the resolved manifest path, ordered layer paths, and merged `.env` block. |

Set `CLAUDE_SESSION_CONFIG_DIR` to relocate the whole config tree.

## Global config file

`config.env` stays dotenv. It is data-only: `KEY=VALUE` assignments, `#` comments, blank lines. Do not put shell pipelines, substitutions, or conditionals in it.

Minimal example:

```dotenv
CLAUDE_SESSION_REAL_CLAUDE=/path/to/claude
```

Profile-specific Claude-side vars (e.g. `CLAUDE_CODE_USE_VERTEX`, `ANTHROPIC_VERTEX_PROJECT_ID`) live in layer JSON `env` blocks, not in `config.env`.

## Manifest and layer model

Profile manifests live under `profiles/` and are the source of truth for profile names. The filename stem is the profile name.

```yaml
# profiles/vertex.yaml
settings-layers:
  - base
  - vertex
```

Layer files live under `settings/`. The filename stem is the layer name.

```json
{
  "env": {
    "CLAUDE_CODE_USE_VERTEX": "1",
    "CLOUD_ML_REGION": "us-central1",
    "ANTHROPIC_VERTEX_PROJECT_ID": "<your-gcp-project-id>"
  }
}
```

Layer names must match `^[A-Za-z0-9._-]+$`. They resolve only against `$CLAUDE_SESSION_CONFIG_DIR/settings/<name>.json`.

## Composition

When a manifest is active, `claude-session` composes its layers in order
with `jq -s`, later layers winning over earlier layers. The versioned
`settings/<layer>.json` files are the sole composition input, so each
profile is fully isolated — no profile can see another's composed
settings.

- The merged output is validated with `jq empty`.
- The merged `.env` block defaults to `{}` when absent.
- A non-object merged `.env` block is an error.

The versioned `settings/<layer>.json` files are the durable source of
truth. Keys such as `effortLevel` re-apply on every launch from their
layer. Claude Code's in-session pickers (`/effort`, `/model`,
`/output-style`) mutate the ephemeral `$session_dir/settings.json` for
the live session only; those writes are discarded when the session dir
is torn down and never leak into the next session.

## Env propagation

All profile env now lives in JSON layer `env` blocks:

- Claude Code env such as `CLAUDE_CODE_*` and `ANTHROPIC_*`
- Wrapper env such as `CLAUDE_SESSION_POST_EXIT_CMD`

After composition, `claude-session` exports each merged `env` entry into its own process before it reads any `CLAUDE_SESSION_*` runtime knobs. That lets a profile override:

- `CLAUDE_SESSION_SHARED_DIR`
- `CLAUDE_SESSION_SYNC_FILES`
- `CLAUDE_SESSION_LINK_FILES`
- `CLAUDE_SESSION_HOME_LINK_FILES`
- `CLAUDE_SESSION_LINK_DIRS`
- `CLAUDE_SESSION_POST_EXIT_CMD`
- `CLAUDE_SESSION_AUTO_TRUST_CWD`

A profile can therefore disable the trust-dialog auto-seed
(`CLAUDE_SESSION_AUTO_TRUST_CWD=0`) or extend the set of files linked
from `$HOME` (`CLAUDE_SESSION_HOME_LINK_FILES=.claude.json:other.json`)
without touching `config.env`.

Empty-string overrides are preserved. For example, `CLAUDE_SESSION_POST_EXIT_CMD=""` disables the post-exit hook for that profile.

Tabs and newlines inside merged `env` values are preserved end-to-end (the
compose sidecar is JSON, and the `apply_profile_env` / `profile show` /
`doctor --verbose` streams use base64 to keep arbitrary bytes intact).

## Resolution semantics

1. Explicit `--profile X` or `CLAUDE_SESSION_PROFILE=X`: require `profiles/X.yaml`; missing manifests exit `6`.
2. Implicit selection with `profiles/default.yaml` present: load `default`.
3. Implicit selection with no `profiles/default.yaml`: **stock mode**.

## Stock mode

Stock mode keeps per-terminal isolation but skips profile composition entirely:

- no `settings.json` is written
- no profile `env` is applied
- `CLAUDE_CONFIG_DIR` still points at the per-terminal session dir
- sync/link behavior, session metadata, and hooks still work from global config/env

If the session dir already contains an old composed `settings.json`, `claude-session` removes the old composed files before launching in stock mode.

## Environment variable reference

| Variable | Default | Meaning |
|----------|---------|---------|
| `CLAUDE_SESSION_CONFIG_DIR` | `$XDG_CONFIG_HOME/claude-session` | Root of `config.env`, `profiles/`, and `settings/`. |
| `CLAUDE_SESSION_SHARED_DIR` | `$HOME/.claude` | Shared Claude config dir staged into the session dir. The lock at `$CLAUDE_SESSION_SHARED_DIR/.claude-session.lock` must resolve to the same host inode across every PTS and container that shares `$HOME/.claude.json`. The dctl default (bind-mounting `~/.claude/` as a directory) satisfies this automatically. Overriding this var to a per-container path breaks cross-container serialization of auto-trust writes. |
| `CLAUDE_SESSION_PROFILE` | empty unless explicit/default manifest resolves | Requested profile name. Explicit values require `profiles/<name>.yaml`. |
| `CLAUDE_SESSION_REAL_CLAUDE` | (auto-discovered) | Absolute path to the real `claude` binary. |
| `CLAUDE_SESSION_POST_EXIT_CMD` | (unset) | Post-exit command. Usually supplied by a profile layer `env` block. |
| `CLAUDE_SESSION_SYNC_FILES` | `.credentials.json:mcp-needs-auth-cache.json` | Colon-separated files copied in/out of the session dir. |
| `CLAUDE_SESSION_LINK_FILES` | `settings.local.json:keybindings.json:CLAUDE.md` | Colon-separated files symlinked into the session dir. |
| `CLAUDE_SESSION_HOME_LINK_FILES` | `.claude.json` | Colon-separated files symlinked from the session dir to `$HOME/<name>`. Each missing target is seeded as `{}` (mode 600) inside a `flock`-protected critical section before Claude Code is exec'd, so concurrent first-launch terminals never race each other's writes and `--dry-run` does not mutate `$HOME`. Used for state vanilla `claude` also reads (trust dialog, project onboarding). Writes to each home-link target use `cs::fn::write_home_link_file`, which atomically renames on regular files and falls back to a locked in-place rewrite when the target is a bind-mount leaf. |
| `CLAUDE_SESSION_LINK_DIRS` | `skills:agents:rules:commands:hooks:plugins` | Colon-separated directories symlinked into the session dir. |
| `CLAUDE_SESSION_AUTO_TRUST_CWD` | `1` | When `1`, the wrapper seeds `projects["$PWD"].hasTrustDialogAccepted=true` and `hasCompletedProjectOnboarding=true` into `$HOME/.claude.json` on each launch, suppressing the "Trust this directory" prompt. Set to `0` to disable. |
| `CLAUDE_SESSION_VERBOSE` | `0` | `1` enables debug logging. |

## Authentication

OAuth-token resolution, secret-store recipes (gopass, pass, Bitwarden, 1Password, age), the `--bare` caveat, and the devcontainer host-passthrough pattern are documented in [auth.md](auth.md). The wrapper itself does not run an auth hook — `CLAUDE_CODE_OAUTH_TOKEN` flows through from the parent environment unchanged.

## Secrets discipline

- Never pass secrets as flags.
- Keep `config.env` and any sensitive layer JSON readable only by the user.
- `profile show` and `doctor --verbose` redact values of `*_TOKEN`, `*_SECRET`, `*_KEY`, `*_PASSWORD` unless explicitly requested to reveal them.

## Missing-config behavior

- Missing `config.env`: treated as empty.
- Missing explicit `profiles/<name>.yaml`: exit `6` with a three-part error.
- Invalid manifest YAML or invalid layer JSON: exit `3`.
- Missing `yq` with manifests present: exit `3` in runtime paths; `doctor` reports the dependency status explicitly.
- Post-exit hook exiting non-zero: warn-only, never fatal.

## `yq` dependency

When any `profiles/*.yaml` file exists, `yq` (mikefarah v4+) is required. `claude-session doctor` always prints a `yq` status row:

- `OK` when `yq` is present
- `WARN` when `yq` is missing but no manifests exist
- `FAIL` when `yq` is missing and manifests do exist

Install hint: <https://github.com/mikefarah/yq>

## Migration from the old profile model

The old model used:

- `profiles/<name>.env`
- `profiles/<name>.settings.json`
- shared-dir `settings.<profile>.json`

That model is removed. Migrate each profile into:

1. `profiles/<name>.yaml`
2. one or more `settings/<layer>.json` files
3. `env` blocks inside those JSON layers for both wrapper and Claude-side env
