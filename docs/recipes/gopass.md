# Recipe: OAuth via `gopass`

End-to-end walkthrough for wiring [gopass](https://www.gopass.pw/) into
`CLAUDE_SESSION_OAUTH_CMD`. See [`auth.md`](../auth.md) for the canonical
contract; this page is the practical setup path.

## Prerequisites

- `gopass` and `gnupg` installed via your distro's package manager
  (`apt install gopass gnupg`, `dnf install gopass gnupg2`,
  `pacman -S gopass gnupg`, `brew install gopass gnupg`, etc.).
- `timeout(1)` on `PATH` (GNU coreutils). The wrapper caps the hook at
  5 seconds; without `timeout(1)`, `doctor` warns and the cap is
  disabled ([`auth.md`](../auth.md) §"OAuth hook contract").
- A working GPG key. If you do not have one:
  ```sh
  gpg --full-generate-key
  ```

## Steps

### 1. Initialise the gopass store

```sh
gopass setup                 # interactive: pick the GPG key to encrypt with
gopass ls                    # confirm the store is healthy
```

For a remote-synced store, use `gopass setup --remote <git-url>` instead.

### 2. Store the OAuth token

Grab the raw token (no `Bearer ` prefix, no JSON wrapping, no trailing
whitespace — see [`auth.md`](../auth.md) §"Pick a secret store"). If you
have already logged in once with `claude /login`, the token lives in
`~/.claude/.credentials.json`.

```sh
gopass insert claude/oauth-token
# paste the token, press Enter
```

Pick any path you like; `claude/oauth-token` is the convention used
throughout the docs.

### 3. Wire the hook into a profile layer

Edit your `base.json` layer (or whichever layer should carry the
default credential lookup):

```json
// $XDG_CONFIG_HOME/claude-session/settings/base.json
{
  "env": {
    "CLAUDE_SESSION_OAUTH_CMD": "gopass show -o claude/oauth-token"
  }
}
```

**Always use `gopass show -o`** (the `-o` flag prints only the secret's
first line and strips the password-store metadata footer). Plain
`gopass show <path>` adds bytes the wrapper will not trim — only `\r`
and `\n` are stripped from the tail ([`auth.md`](../auth.md) §"OAuth
hook contract", row "Success").

Tighten permissions on layers that reference your store:

```sh
chmod 600 "$XDG_CONFIG_HOME/claude-session/settings/base.json"
```

### 4. Disable the hook on profiles that use a different auth path

For profiles using e.g. Vertex AI service-account auth, override the
hook to the empty string in that layer so the wrapper skips the lookup
([`auth.md`](../auth.md) §"Per-profile disable"):

```json
// settings/vertex.json
{
  "env": {
    "CLAUDE_SESSION_OAUTH_CMD": ""
  }
}
```

### 5. Export `GPG_TTY` in your shell rc

This is the single most common cause of "hook works interactively, fails
under the wrapper". `gpg-agent` needs a TTY to prompt for the
passphrase, and the wrapper's child process will not inherit one unless
your shell exports it ([`auth.md`](../auth.md) §"Security checklist",
last bullet).

```sh
# ~/.bashrc, ~/.zshrc, or equivalent
export GPG_TTY=$(tty)
```

If you launch `claude-session` from a non-TTY context (cron, systemd
unit, editor hook), also configure a graphical pinentry in
`~/.gnupg/gpg-agent.conf` (`pinentry-gnome3`, `pinentry-qt`, etc.) and
reload the agent:

```sh
gpgconf --reload gpg-agent
```

### 6. Verify

Run these in order ([`auth.md`](../auth.md) §"Verify"):

```sh
# Hook is bound to the active profile:
claude-session --dry-run run | grep oauth_hook
# → oauth_hook=gopass show -o claude/oauth-token

# Redaction works:
claude-session profile show default
# → CLAUDE_SESSION_OAUTH_CMD=<redacted>
claude-session profile show default --verbose
# → CLAUDE_SESSION_OAUTH_CMD=gopass show -o claude/oauth-token

# Dependencies look healthy:
claude-session doctor | grep -E 'oauth|timeout'
# oauth hook  OK  configured
# timeout     OK  /usr/bin/timeout

# Smoke-test the hook itself outside the wrapper:
gopass show -o claude/oauth-token | wc -c     # > 0, no stray bytes

# Run for real with verbose stderr if the hook misbehaves:
CLAUDE_SESSION_VERBOSE=1 claude-session
```

For any profile that disables the hook (step 4):

```sh
claude-session --profile vertex --dry-run run | grep oauth_hook
# → oauth_hook=          (empty — skipped)
```

### 7. (Optional) Drop the local credential file

Once the hook resolves the token, you can delete the upstream-managed
credential cache so the next launch re-seeds it from `gopass`:

```sh
rm ~/.claude/.credentials.json
```

The next `claude-session` invocation will export
`CLAUDE_CODE_OAUTH_TOKEN` from the hook output and pass it to the child
`claude` process ([`auth.md`](../auth.md) §"Where the token lives at
runtime").

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `warning: hook command failed: gopass show -o …` | `gpg-agent` is locked, `GPG_TTY` is unset, or the entry does not exist. | `CLAUDE_SESSION_VERBOSE=1 claude-session` to see the hook's stderr. Confirm `gopass show -o <path>` works in a fresh shell. Export `GPG_TTY=$(tty)`. |
| Token has stray bytes / upstream auth fails | Used `gopass show` instead of `gopass show -o`. | Switch to `-o`, or pipe through `tr -d '\r\n'` in the hook command. |
| `doctor` reports `timeout not found` | GNU coreutils not installed. | Install coreutils, or accept the missing-cap risk. |
| Works as your user, fails as another user / in cron | `gpg-agent` socket is not reachable. | Either start an agent in the cron environment or use a non-interactive pinentry; see GnuPG docs. |
| `--bare` mode reports "Not logged in" | Upstream `--bare` ignores OAuth tokens. | Store an API key in gopass too and use `ANTHROPIC_API_KEY="$(gopass show -o claude/api-key)" claude-session --bare …` ([`auth.md`](../auth.md) §"`--bare` caveat"). |

## See also

- [`../auth.md`](../auth.md) — canonical OAuth hook contract and
  cross-backend reference.
- [`../config.md`](../config.md) — layer composition rules and env
  precedence.
