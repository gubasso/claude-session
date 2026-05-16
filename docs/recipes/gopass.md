# Recipe: OAuth via `gopass`

Resolve the long-lived Claude OAuth token from
[gopass](https://www.gopass.pw/) on the host and hand it to
`claude-session` as `CLAUDE_CODE_OAUTH_TOKEN`. The wrapper's
token-precedence step 1 ([`auth.md`](../auth.md) §"Token precedence")
picks up the pre-set env var directly.

See [`auth.md`](../auth.md) for precedence rules and alternative
secret-store backends.

## Prerequisites

- `gopass` and `gnupg` installed via your distro's package manager
  (`apt install gopass gnupg`, `dnf install gopass gnupg2`,
  `pacman -S gopass gnupg`, `brew install gopass gnupg`).
- A working GPG key. If you do not have one:
  ```sh
  gpg --full-generate-key
  ```

## Steps

### 1. Initialise the gopass store

```sh
gopass setup       # interactive: pick the GPG key to encrypt with
gopass ls          # confirm the store is healthy
```

For a remote-synced store, use `gopass setup --remote <git-url>`.

### 2. Generate and store the OAuth token

Mint a long-lived (~1 year) OAuth token with the upstream `claude`
binary:

```sh
claude setup-token
```

The command walks you through OAuth authorization in the browser and
prints the token to the terminal. It saves nothing — copy the value
before closing the terminal. The token is subscription-bound
(Pro/Max/Team/Enterprise) and bills the same way interactive `/login`
does; it is not an API key (see [`auth.md`](../auth.md) §"What kind
of token to store").

Insert it into gopass:

```sh
gopass insert claude/oauth-token
# paste the token, press Enter
```

> **Do not** paste the `accessToken` field from
> `~/.claude/.credentials.json`. That value is a short-lived access
> token (hours), not the long-lived setup-token. Tokens passed via
> env are not refreshed, so you would have to re-insert daily.

Pick any path you like; `claude/oauth-token` is the convention used
throughout the docs.

### 3. Export the token from your shell rc

```sh
# ~/.bashrc, ~/.zshrc, or equivalent
export GPG_TTY=$(tty)
export CLAUDE_CODE_OAUTH_TOKEN="$(gopass show -o claude/oauth-token)"
```

That's the entire integration. New shells get the token; `claude-session`
sees `CLAUDE_CODE_OAUTH_TOKEN` already set and inherits it into the
child `claude` process unchanged.

Notes:

- **`gopass show -o`** prints only the secret's first line — no
  password-store metadata footer. Always use the `-o` flag.
- **`GPG_TTY=$(tty)`** lets `gpg-agent` find a pinentry on first
  unlock. Without it, the export above may hang or fail.
- For non-TTY launch contexts (cron, systemd unit, editor hook),
  configure a graphical pinentry in `~/.gnupg/gpg-agent.conf`
  (`pinentry-gnome3`, `pinentry-qt`, etc.) and reload the agent:
  ```sh
  gpgconf --reload gpg-agent
  ```

### 4. (Containers) Forward the token into a devcontainer

When `claude-session` runs inside a container, resolve the token on
the host and forward it through devcontainer-CLI's `${localEnv:…}`
substitution — keep gopass, GPG, and your password store on the host
where they belong.

```jsonc
// .devcontainer/devcontainer.json
{
  "containerEnv": {
    "CLAUDE_CODE_OAUTH_TOKEN": "${localEnv:CLAUDE_CODE_OAUTH_TOKEN}"
  }
}
```

Devcontainer-CLI resolves `${localEnv:…}` on the host at
container-create time. Inside the container, `claude-session` sees
the env var already set and uses it directly. No gopass, no GPG, no
`~/.password-store` need exist inside the container.

The substitution runs once at create time, so a rotated token on the
host does not reach an already-running container — recreate
(`devcontainer up --remove-existing-container`, `dctl ws recreate`,
or your tool's equivalent) to pick up a new value.

Trade-off: `containerEnv` makes the token readable by every process
in the container for its lifetime. For YOLO-mode agent setups this
matches how other forwarded secrets typically work, but a hostile or
prompt-injected agent inside the container can read and exfiltrate
it. Mitigations: rotate proactively (every 30–90 days, not the 1-year
expiry), consider a dedicated Claude.ai account for agent workloads,
and restrict container egress where possible.

### 5. Verify

```sh
# Token is set in the current shell:
[[ -n "$CLAUDE_CODE_OAUTH_TOKEN" ]] && echo OK

# Wrapper dry-run runs cleanly with the token in env:
claude-session --dry-run run

# Smoke-test gopass independently:
gopass show -o claude/oauth-token | wc -c     # > 0, no stray bytes
```

Inside a devcontainer, the same `[[ -n "$CLAUDE_CODE_OAUTH_TOKEN" ]]`
check confirms `${localEnv:…}` forwarding resolved correctly.

### 6. Rotate

Re-mint the token every 30–90 days (don't ride the 1-year expiry —
upstream has no self-service revocation today) and immediately on any
suspicion of compromise:

```sh
claude setup-token                       # new token to terminal
gopass insert -f claude/oauth-token      # overwrite stored value
# new shells / new devcontainer instances pick up the rotated token
```

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `gopass show` prompts for a passphrase on every new shell | `gpg-agent` cache TTL expired, or `GPG_TTY` is unset. | Confirm `export GPG_TTY=$(tty)` runs before the token export. Tune `default-cache-ttl` / `max-cache-ttl` in `~/.gnupg/gpg-agent.conf` for a longer cache window. |
| Token has stray bytes / upstream auth fails | Used `gopass show` instead of `gopass show -o`. | Switch to `-o`, or pipe through `tr -d '\r\n'`. |
| Works as your user, fails in cron / under another user | `gpg-agent` socket not reachable in that context. | Start an agent in the cron environment or use a non-interactive pinentry; see GnuPG docs. |
| `claude-session --bare` reports "Not logged in" | Upstream `--bare` ignores OAuth tokens. | Store an API key alongside the OAuth token and use `ANTHROPIC_API_KEY="$(gopass show -o claude/api-key)" claude-session --bare …` ([`auth.md`](../auth.md) §"`--bare` caveat"). |
| Auth fails after months | Token expired or was revoked. | Re-run step 6 (rotate). |
| Container starts but token is unset inside | Host shell rc did not export `CLAUDE_CODE_OAUTH_TOKEN` before the container was launched, so `${localEnv:…}` resolved to empty. | Start a new shell that sources the rc, or `export` the token manually before `dctl ws up` / `devcontainer up`. |

## See also

- [`../auth.md`](../auth.md) — canonical token precedence and
  cross-backend reference.
- [`../config.md`](../config.md) — layer composition rules and env
  precedence.
