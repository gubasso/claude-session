# Authentication

How `claude-session` supplies an OAuth token (or API key) to the upstream
`claude` binary, and how to wire your own secret-store lookup into a profile.

## Token precedence

The wrapper resolves credentials in this order (highest priority first):

1. **`CLAUDE_CODE_OAUTH_TOKEN`** already set in the environment (e.g.
   exported from your shell rc) — the wrapper does **not** run a hook.
2. **`CLAUDE_SESSION_OAUTH_CMD`** — the OAuth-lookup hook. Stdout becomes
   `CLAUDE_CODE_OAUTH_TOKEN` for the child `claude` process.
3. **Native auth** — when the hook is unset or its output is empty, the
   wrapper falls through to the upstream binary's own auth (`~/.claude/.credentials.json`,
   keychain, interactive `/login`, `ANTHROPIC_API_KEY`, `apiKeyHelper`).

A hook *failure* is **never fatal** — the wrapper logs a warning and falls
through to native auth.

## The OAuth hook contract

`CLAUDE_SESSION_OAUTH_CMD` is a shell command line. The wrapper runs it via
`bash -c "$cmd"` (so pipes, env-var interpolation, and shell builtins are
all available) under a 5-second `timeout(1)` cap.

| Aspect | Behavior |
|---|---|
| Invoked when | `bare=0` (no `--bare`), `CLAUDE_CODE_OAUTH_TOKEN` is unset, and `CLAUDE_SESSION_OAUTH_CMD` is non-empty. |
| Timeout | 5 seconds. If `timeout(1)` is missing, the cap is silently disabled (`doctor` warns). |
| Success | Exit 0 AND stdout has at least one non-whitespace byte. The wrapper trims trailing `\r`/`\n` and exports `CLAUDE_CODE_OAUTH_TOKEN=<stdout>`. |
| Failure | Non-zero exit, or empty/whitespace-only stdout. The wrapper logs a one-line warning to stderr and falls through to native auth. With `CLAUDE_SESSION_VERBOSE=1`, the hook's stderr is dumped too. |
| Working dir | The wrapper's CWD at startup (your terminal's CWD when you typed `claude-session`). |
| Environment | The wrapper's full env at the moment the hook runs (post-composition, so any `CLAUDE_SESSION_*` set by the active profile's `env` block is visible). |

## Setup walkthrough

### 1. Pick a secret store

Any tool whose CLI prints the token to stdout works. Common picks:

| Tool | Lookup command shape |
|---|---|
| [gopass](https://www.gopass.pw/) | `gopass show -o <path>` |
| [pass](https://www.passwordstore.org/) | `pass show <path>` |
| [Bitwarden CLI](https://bitwarden.com/help/cli/) | `bw get password <item-name-or-id>` |
| [1Password CLI](https://developer.1password.com/docs/cli) | `op read "op://<vault>/<item>/credential"` |
| [age](https://age-encryption.org/) | `age -d -i <key-file> <encrypted-token-file>` |
| Plain file (last resort) | `cat <path-to-token-file>` |

Whichever you pick, **store only the raw token** (no surrounding whitespace,
no `Bearer ` prefix, no JSON wrapping). The wrapper takes stdout verbatim
modulo trailing newline trimming.

### 2. Choose where to put it

The hook command lives in the `env` block of a profile **layer** JSON, NOT
in `config.env`. (`config.env` is for global wrapper config that does not
vary by profile.)

Typical pattern: put OAuth in your `base.json` so every profile inherits it,
and override it to `""` in profile-specific layers that should use a
different auth path (e.g. Vertex AI service-account auth).

### 3. Edit the layer JSON

```json
// $XDG_CONFIG_HOME/claude-session/settings/base.json
{
  "env": {
    "CLAUDE_SESSION_OAUTH_CMD": "gopass show -o claude/oauth-token"
  }
}
```

### 4. Verify

```sh
# 1. Confirm the hook is configured for the active profile.
claude-session --dry-run run | grep oauth_hook
# → oauth_hook=gopass show -o claude/oauth-token

# 2. Confirm the redaction policy hides it from `profile show`.
claude-session profile show <profile>
# → CLAUDE_SESSION_OAUTH_CMD=<redacted>

# Reveal it explicitly when you need to debug:
claude-session profile show <profile> --verbose
# → CLAUDE_SESSION_OAUTH_CMD=gopass show -o claude/oauth-token

# 3. Confirm doctor sees the dependency stack.
claude-session doctor | grep -E 'oauth|timeout'
# → oauth hook    OK    configured
# → timeout       OK    /usr/bin/timeout

# 4. Run for real with verbose logging if the hook misbehaves.
CLAUDE_SESSION_VERBOSE=1 claude-session
```

## Recipes by backend

Drop the `command` value into the `env` block of the chosen layer JSON.

```json
// gopass
"CLAUDE_SESSION_OAUTH_CMD": "gopass show -o <path/to/secret>"

// pass
"CLAUDE_SESSION_OAUTH_CMD": "pass show <path/to/secret>"

// Bitwarden (assumes BW_SESSION already exported in your shell rc)
"CLAUDE_SESSION_OAUTH_CMD": "bw get password <item>"

// 1Password
"CLAUDE_SESSION_OAUTH_CMD": "op read 'op://Personal/Anthropic/credential'"

// age-encrypted file
"CLAUDE_SESSION_OAUTH_CMD": "age -d -i ~/.config/age/keys.txt ~/.config/secrets/anthropic.age"

// Plain file (chmod 600, only acceptable if disk is encrypted)
"CLAUDE_SESSION_OAUTH_CMD": "cat ~/.config/secrets/anthropic-token"
```

Multi-step pipelines work; the wrapper runs the value through `bash -c`:

```json
"CLAUDE_SESSION_OAUTH_CMD": "op read 'op://Personal/Anthropic/credential' | tr -d '\\r\\n'"
```

Backslashes inside JSON strings need escaping (`\\r\\n`, not `\r\n`).

## Per-profile disable

To disable the OAuth hook for a specific profile (e.g. one that uses
Vertex AI service-account auth), set the var to the **empty string** in
that profile's layer:

```json
// settings/vertex.json
{
  "env": {
    "CLAUDE_CODE_USE_VERTEX": "1",
    "ANTHROPIC_VERTEX_PROJECT_ID": "<your-gcp-project>",
    "CLOUD_ML_REGION": "<your-region>",
    "CLAUDE_SESSION_OAUTH_CMD": ""
  }
}
```

Layer composition is last-wins, so `vertex.yaml`'s `[base, vertex]` order
overrides `base.json`'s OAuth command with the empty string. The wrapper's
hook block treats empty as "unset" (`[[ -n "${VAR:-}" ]]`) and skips the
lookup entirely.

## Where the token lives at runtime

- The wrapper writes the token to **`CLAUDE_CODE_OAUTH_TOKEN` in its own
  process env**. It is never written to a file under the session dir or
  the shared dir. It is inherited by the child `claude` process.
- It is NOT persisted to `~/.claude/.credentials.json` — that file is
  managed by the upstream binary's own auth flow (e.g. `claude /login`).
- `session-meta.json` does NOT include the token.
- `profile show --verbose` reveals the *lookup command* but never the
  resolved token (the wrapper does not run the hook from `profile show`).

## `--bare` caveat

The upstream `claude --bare` mode (added in v2.1.81 for fast scripted
`-p` calls) **deliberately disables OAuth and keychain auth**. With
`--bare`, the upstream binary requires an Anthropic API key via
`ANTHROPIC_API_KEY` or via an `apiKeyHelper` defined in a `--settings`
overlay. OAuth tokens in `.credentials.json` and the
`CLAUDE_CODE_OAUTH_TOKEN` env var are intentionally ignored.

This means `CLAUDE_SESSION_OAUTH_CMD` is a no-op for
`claude-session --bare …`. Two supported workarounds:

```sh
# 1. Inline API key via your secret store:
export ANTHROPIC_API_KEY
ANTHROPIC_API_KEY="$(pass show path/to/key)" \
  claude-session --bare -p --model haiku "ping"

# 2. Or wire apiKeyHelper into a profile layer so composition picks it up:
#    settings/<layer>.json:
#      { "apiKeyHelper": "pass show path/to/key" }
```

If you do not need `--bare`'s startup-time savings, drop the flag and the
wrapper's normal OAuth flow works as documented above.

## Security checklist

- **Never put the literal token in `config.env` or any `settings/*.json`.**
  Use a hook command that fetches it on demand from a secret store.
- **`chmod 600` your layer JSONs** if any of them embeds a path to a
  secret store. The hook *command* is not itself a secret, but the
  filename it points at narrows the attack surface for someone with
  read access to your `$XDG_CONFIG_HOME`.
- **Don't enable `apiKeyHelper` for tokens that an OAuth hook can
  supply.** Pick one path. Mixing the two is fragile and surprising.
- **Never write a token to disk under `$XDG_RUNTIME_DIR/claude-session/`
  or `$XDG_STATE_HOME/claude-session/`.** The session dirs are sync
  targets and may be cleaned by `session clean`.
- **`CLAUDE_SESSION_OAUTH_CMD` is redacted by default** in `profile show`
  and `doctor --verbose` output (`<redacted>`). Pass `--verbose` to
  reveal it when debugging. The redaction matcher is suffix-based
  (`*_TOKEN`, `*_SECRET`, `*_KEY`, `*_PASSWORD`) plus the literal
  `CLAUDE_SESSION_OAUTH_CMD`.
- **Verify the hook in a fresh shell** before relying on it. A hook that
  works in your interactive terminal but fails under `claude-session`
  usually means the hook needs an env var that your shell rc sets but
  the wrapper does not (e.g. `BW_SESSION`, `OP_SESSION_*`,
  `GPG_TTY`). Either export those globally or include them in the
  hook command itself.

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `[claude-session] warning: hook command failed: <cmd>` and `claude` falls through to interactive `/login` | Hook exited non-zero. | Run `CLAUDE_SESSION_VERBOSE=1 claude-session` and inspect the hook's stderr dump. Common causes: locked secret store, missing `BW_SESSION`/`OP_SESSION_*`, missing `gpg-agent`, expired credential. |
| Hook works interactively but not under the wrapper | Hook depends on a session var your shell rc sets that the wrapper's parent shell did not export. | Either export the dependency unconditionally in your shell rc, or embed the unlock step into the hook command. |
| `doctor` shows `oauth hook  WARN  configured, but timeout not found` | `timeout(1)` is not on `PATH`. | Install GNU coreutils (`coreutils`/`coreutils-bin` package) or accept the missing-cap risk. |
| `--bare` mode "Not logged in" | Upstream `--bare` ignores OAuth credentials. | Use `ANTHROPIC_API_KEY` or `apiKeyHelper` (see `--bare` caveat above). |
| Token contains stray bytes that break upstream auth | Hook output has trailing data beyond the token. | The wrapper trims trailing `\r`/`\n` only. Pipe through `tr -d '\r\n'` or `head -c <bytes>` in the hook command if your secret store adds anything else. |

## See also

- [`config.md`](config.md) — global wrapper config; profile env precedence rules.
- [`architecture.md`](architecture.md) §"Session-dir lifecycle" step 6 — exact place in the startup sequence the hook runs.
- [`commands.md`](commands.md) — `doctor` output reference, `profile show` redaction.
