# Authentication

How `claude-session` supplies an OAuth token (or API key) to the
upstream `claude` binary.

## Token precedence

The wrapper itself does not impose an auth precedence. It only does
env passthrough: every variable set in the wrapper's parent
environment (including any auth-related one) is inherited by the
child `claude` process via normal exec. From the wrapper's point of
view there are two states:

1. **`CLAUDE_CODE_OAUTH_TOKEN` is set** in the environment (e.g.
   exported from your shell rc, or forwarded from a host into a
   container). It flows through unchanged — nothing
   wrapper-specific happens. This is the supported path for
   subscription auth.
2. **`CLAUDE_CODE_OAUTH_TOKEN` is unset** — the upstream binary
   handles auth itself (`~/.claude/.credentials.json`, keychain,
   interactive `/login`, `ANTHROPIC_API_KEY`, `apiKeyHelper`,
   cloud-provider auth) with its own internal precedence.

If you set both `CLAUDE_CODE_OAUTH_TOKEN` and an API-key variable
(`ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `apiKeyHelper`, Vertex
or Bedrock auth), the wrapper passes all of them through and the
upstream binary decides which one wins. To avoid surprises (e.g. API
billing instead of subscription billing), pick one auth path per
shell / per profile.

The single supported pattern for subscription users is: put a
long-lived `claude setup-token` value into your preferred secret
store, export it from your shell rc as `CLAUDE_CODE_OAUTH_TOKEN`, and
let the upstream binary pick it up. Inside containers, forward the
env via the container tool's host-passthrough mechanism (see
"Devcontainers and remote shells" below).

## What kind of token to store

`CLAUDE_CODE_OAUTH_TOKEN` expects a **long-lived OAuth token** minted
by the upstream `claude` CLI:

```sh
claude setup-token
```

This command walks through OAuth authorization in a browser and prints
a ~1-year token to the terminal. It saves nothing — copy the value
before closing the terminal, then store it in your secret backend.

The token is **subscription-bound** (Pro/Max/Team/Enterprise) and
bills the same way interactive `/login` does. It is not an API key,
and it does not consume API credits. Tokens have a `sk-ant-oat01-`
wire prefix (vs `sk-ant-api03-` for API keys).

> **Do not** paste the `accessToken` field from
> `~/.claude/.credentials.json` into your secret store. That value is
> a short-lived access token (hours), not the long-lived setup-token.
> Tokens supplied via env are not refreshed, so you would have to
> re-insert daily.

Rotation: don't ride the full 1-year expiry. Re-run `claude
setup-token` and re-insert into your store every 30–90 days, and
immediately on any suspicion of compromise. Upstream has no
self-service revocation dashboard today, so rotation cadence is the
primary control.

## Pick a secret store

Any tool whose CLI prints the token to stdout works. Common picks:

| Tool | Lookup command shape |
|---|---|
| [gopass](https://www.gopass.pw/) | `gopass show -o <path>` |
| [pass](https://www.passwordstore.org/) | `pass show <path>` |
| [Bitwarden CLI](https://bitwarden.com/help/cli/) | `bw get password <item-name-or-id>` |
| [1Password CLI](https://developer.1password.com/docs/cli) | `op read "op://<vault>/<item>/credential"` |
| [age](https://age-encryption.org/) | `age -d -i <key-file> <encrypted-token-file>` |
| Plain file (last resort) | `cat <path-to-token-file>` |

Whichever you pick, **store only the raw token** (no surrounding
whitespace, no `Bearer ` prefix, no JSON wrapping).

## Wire it into your shell rc

Export `CLAUDE_CODE_OAUTH_TOKEN` from your store, in your shell rc, so
every new shell sees it:

```sh
# ~/.bashrc, ~/.zshrc, or equivalent
export CLAUDE_CODE_OAUTH_TOKEN="$(<your-lookup-command>)"
```

Backend-specific snippets:

```sh
# gopass
export GPG_TTY=$(tty)
export CLAUDE_CODE_OAUTH_TOKEN="$(gopass show -o claude/oauth-token)"

# pass
export CLAUDE_CODE_OAUTH_TOKEN="$(pass show claude/oauth-token)"

# Bitwarden (assumes BW_SESSION already exported)
export CLAUDE_CODE_OAUTH_TOKEN="$(bw get password anthropic-oauth)"

# 1Password
export CLAUDE_CODE_OAUTH_TOKEN="$(op read 'op://Personal/Anthropic/credential')"

# age-encrypted file
export CLAUDE_CODE_OAUTH_TOKEN="$(age -d -i ~/.config/age/keys.txt ~/.config/secrets/anthropic.age)"

# Plain file (chmod 600, only acceptable if the disk is encrypted)
export CLAUDE_CODE_OAUTH_TOKEN="$(cat ~/.config/secrets/anthropic-token)"
```

Multi-step pipelines work — it is just a shell `$()` substitution:

```sh
export CLAUDE_CODE_OAUTH_TOKEN="$(op read 'op://Personal/Anthropic/credential' | tr -d '\r\n')"
```

## Verify

```sh
# Token is set in the current shell:
[[ -n "$CLAUDE_CODE_OAUTH_TOKEN" ]] && echo OK

# Smoke-test your secret-store command independently:
<your-lookup-command> | wc -c    # > 0, no stray bytes

# Sanity-check the wrapper sees it (no warning, normal dry-run output):
claude-session --dry-run run
```

## Where the token lives at runtime

- On the host (or any non-containerized shell), the token lives only
  in the parent shell's process environment and is inherited by the
  child `claude` process via normal exec. It is never written to a
  file under the session dir or the shared dir, and never recorded in
  `session-meta.json`.
- It is NOT persisted to `~/.claude/.credentials.json` — that file is
  managed by the upstream binary's own auth flow (e.g. `claude /login`).
- Inside a container created with the `containerEnv` pattern below,
  the token additionally lives in the container's environment for the
  container's lifetime — readable by every process the container
  starts. Updating the host-side value does *not* propagate to a
  running container; you must recreate (or rebuild) the container for
  the new value to take effect. See "Trade-offs" below.

## Devcontainers and remote shells

When `claude-session` runs inside a container (devcontainer, codespace,
remote SSH, etc.), the secret store's dependencies — `gpg-agent`, the
GPG key, the password store, `BW_SESSION`, etc. — would need to be
available *inside* that container, which is fragile and expands the
attack surface.

The recommended pattern is to **resolve the token on the host and
forward it as `CLAUDE_CODE_OAUTH_TOKEN`**. Precedence step 1 picks it
up directly inside the container, so the container needs no
secret-store tooling at all.

For VS Code / `devcontainer.json`-style containers, the canonical
pattern is the `${localEnv:…}` substitution, which the devcontainer
CLI resolves on the host at container-create time:

```jsonc
// .devcontainer/devcontainer.json
{
  "containerEnv": {
    "CLAUDE_CODE_OAUTH_TOKEN": "${localEnv:CLAUDE_CODE_OAUTH_TOKEN}"
  }
}
```

Then export the token in your host shell rc so the substitution
resolves to a real value:

```sh
# ~/.bashrc, ~/.zshrc, or equivalent
export CLAUDE_CODE_OAUTH_TOKEN="$(<your-secret-store-command>)"
# e.g. export CLAUDE_CODE_OAUTH_TOKEN="$(gopass show -o claude/oauth-token)"
```

Other remote shells follow the same principle — set
`CLAUDE_CODE_OAUTH_TOKEN` in the remote env however the tool exposes,
and `claude-session` will pick it up.

### Trade-offs

- `containerEnv` injects the token into the container's environment
  for the container's full lifetime. Every process the container
  starts can read it via `env`, `/proc/<pid>/environ`, etc. In
  YOLO-mode agent setups this matches how other forwarded secrets
  (`GH_TOKEN`, `GITLAB_TOKEN`, …) typically work, but a hostile or
  prompt-injected agent inside the container can exfiltrate it.
- Mitigations: rotate frequently (see "What kind of token to store"
  above); consider a dedicated Claude.ai account for agent workloads
  to cap the blast radius; restrict container egress to required
  hosts; add `PreToolUse` hooks to block obvious exfil patterns.
- If your container tool supports per-exec env injection (rather than
  container-lifetime env), prefer that over `containerEnv` — it
  narrows the window during which the token is in-container.

## `--bare` caveat

The upstream `claude --bare` mode (added in v2.1.81 for fast scripted
`-p` calls) **deliberately disables OAuth and keychain auth**. With
`--bare`, the upstream binary requires an Anthropic API key via
`ANTHROPIC_API_KEY` or via an `apiKeyHelper` defined in a `--settings`
overlay. OAuth tokens in `.credentials.json` and the
`CLAUDE_CODE_OAUTH_TOKEN` env var are intentionally ignored.

Two supported workarounds:

```sh
# 1. Inline API key from your secret store. Resolve it into a local
#    variable first so the value is never embedded as a literal in
#    your shell history:
key=$(pass show claude/api-key)
ANTHROPIC_API_KEY=$key claude-session --bare -p --model haiku "ping"

# 2. Or wire apiKeyHelper into a profile layer so composition picks it up:
#    settings/<layer>.json:
#      { "apiKeyHelper": "pass show claude/api-key" }
```

If you do not need `--bare`'s startup-time savings, drop the flag and
the wrapper's normal OAuth flow works as documented above.

## Security checklist

- **Never put the literal token in `config.env` or any
  `settings/*.json`.** Use a secret-store command in your shell rc
  that fetches it on demand.
- **Don't enable `apiKeyHelper` for tokens that subscription auth can
  supply.** Pick one path. Mixing the two is fragile and surprising.
- **Never write a token to disk under `$XDG_RUNTIME_DIR/claude-session/`
  or `$XDG_STATE_HOME/claude-session/`.** The session dirs are sync
  targets and may be cleaned by `session clean`.
- **Suffix-based redaction.** `profile show` and `doctor --verbose`
  redact values of `*_TOKEN`, `*_SECRET`, `*_KEY`, `*_PASSWORD` by
  default. Pass `--verbose` to reveal them when debugging.
- **Verify the export in a fresh shell** before relying on it. If
  `echo "${CLAUDE_CODE_OAUTH_TOKEN+set}"` is empty in a new terminal,
  your shell rc did not run — fix the rc sourcing first (login vs
  non-login shells, missing `~/.profile` chain, etc.).
- **Rotate the token on a schedule, not on expiry.** The `claude
  setup-token` value is valid for ~1 year, but upstream offers no
  self-service revocation today. Re-mint every 30–90 days and
  immediately on any suspicion of compromise (lost device, anomalous
  usage, untrusted content fed to a YOLO-mode agent).

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `claude` falls through to interactive `/login` despite the export | `CLAUDE_CODE_OAUTH_TOKEN` is empty in the wrapper's parent shell. | Run `echo "${CLAUDE_CODE_OAUTH_TOKEN+set}"` in the same shell. If empty, the rc did not run; ensure the `export` lives in a file that login and non-login shells both source. |
| Secret store works interactively but the export fails | The store needs an env var the rc sets only conditionally (e.g. `BW_SESSION`, `OP_SESSION_*`, `GPG_TTY`). | Either export those unconditionally in the rc, or unlock the store before the token export. |
| `--bare` reports "Not logged in" | Upstream `--bare` ignores OAuth credentials. | Use `ANTHROPIC_API_KEY` or `apiKeyHelper` (see `--bare` caveat above). |
| Token contains stray bytes that break upstream auth | The lookup command's output has trailing data beyond the token. | Pipe through `tr -d '\r\n'` or `head -c <bytes>` in the export command. |
| Container starts but `CLAUDE_CODE_OAUTH_TOKEN` is empty inside | Host shell rc had not run, so `${localEnv:…}` resolved to empty when the container was created. | Open a shell that sources the rc before `dctl ws up` / `devcontainer up`, or export the token explicitly before launching the container. |

## See also

- [`config.md`](config.md) — global wrapper config; profile env precedence rules.
- [`commands.md`](commands.md) — `doctor` and `config show` output reference, redaction.
- [`recipes/gopass.md`](recipes/gopass.md) — full gopass walkthrough.
