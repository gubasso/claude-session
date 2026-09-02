# Supplying your own claude plugins

Every launch gives its terminal an isolated configuration directory, and `claude` keeps everything it knows about plugins inside the directory it was pointed at: which marketplaces are registered, which plugins are installed, and the cached copy of each one. A directory that never existed knows none of it, so the plugins you installed once do not come back.

This guide sets up the one read-only tree that reaches every session, on every account, in every terminal. It is a one-time step per machine. The wrapper writes no user configuration ([ADR-0015](../decisions/ADR-0015-retire-the-init-verb.md)), so nothing does this for you.

Language servers are the usual reason to want it. A code intelligence plugin gives `claude` diagnostics after every edit and go-to-definition instead of grep, and without a seed it has neither in any session.

## Where the tree lives

```text
${XDG_DATA_HOME:-$HOME/.local/share}/claude-session/plugin-seed/
```

Beside the [asset tree](./supplying-child-assets.md), not inside it: that tree holds the names `claude` reads from its own configuration directory, and this is not one of them. [XDG storage](../reference/xdg-storage.md#artifact-table) owns the placement.

## What it holds

The layout `claude` expects of a seed, which is the layout of its own plugins directory:

```text
plugin-seed/
  known_marketplaces.json
  installed_plugins.json
  marketplaces/<name>/...
  cache/<marketplace>/<plugin>/<version>/...
```

The wrapper reads none of it. It sets `CLAUDE_CODE_PLUGIN_SEED_DIR` to the tree and copies the two JSON files into the session directory as opaque bytes ([ADR-0117](../decisions/ADR-0117-supply-plugins-from-a-read-only-seed.md)). You do not set that variable yourself, and one you export is dropped: a session reaches the tree the wrapper selected rather than the one its environment was built for.

## Building it

Build it with `claude`'s own commands, directly at the path it will be read from:

```bash
seed="${XDG_DATA_HOME:-$HOME/.local/share}/claude-session/plugin-seed"
CLAUDE_CODE_PLUGIN_CACHE_DIR="$seed" claude plugin marketplace add anthropics/claude-plugins-official
CLAUDE_CODE_PLUGIN_CACHE_DIR="$seed" claude plugin install pyright-lsp@claude-plugins-official
```

`CLAUDE_CODE_PLUGIN_CACHE_DIR` is the child's own build-time variable for this, and it creates the tree it names. Nothing lands in the configuration directory those two commands otherwise use, so they need no `CLAUDE_CONFIG_DIR` and touch no account of yours.

Build it where it will live, which is what that variable is for. `claude` finds a seeded marketplace by probing the tree, so that half survives a move, but it finds a plugin's cached copy through the absolute path recorded in `installed_plugins.json`. A seed built somewhere else and moved here registers its marketplaces and loads none of its plugins, with no error to say why. Whatever route you take, check the recorded paths before relying on the seed:

```bash
grep -o '"install[A-Za-z]*": "[^"]*"' "$seed"/*.json
```

Every path it prints must be under `$seed`.

Make it read-only if you like. `claude` never writes a seed, and a tree the wrapper cannot modify either is one less thing to wonder about:

```bash
chmod -R a-w "$seed"
```

Restore write before you rebuild it: removing a file needs write permission on the directory holding it, so a later `claude plugin install` into a read-only seed fails.

## Checking it

```bash
claude-session doctor
```

The `session-plugin-seed` check names what a launch would supply. No tree at all is skipped rather than reported as a defect: most setups keep no plugins, and an absent seed is a feature not in use. A tree that exists but holds neither JSON file is a warning, because that is a seed built wrongly rather than one you do not have.

After a launch, the state appears inside the session directory:

```bash
ls -la "${XDG_STATE_HOME:-$HOME/.local/state}"/claude-session/accounts/*/sessions/*/*/plugins/
```

## What a session does not keep

A plugin installed from inside a session is installed into that session's own directory, and it goes when the session does. Install it into the seed instead, by the steps above, and every later session has it.

The same is true of `/plugin disable` and of anything else `claude` records about a plugin. The seed is the durable answer; the session's copy of it is scratch.

## Stopping the recommendation

`claude` offers to install a language-server plugin when it sees a file whose server is on your `PATH`. A session directory that never existed has no record of your answer, so the offer arrives again in every session, and answering it installs into a directory that is about to be discarded.

Set the key once, in `${XDG_CONFIG_HOME:-$HOME/.config}/claude-session/config.toml`:

```toml
suppress_lsp_recommendations = true
```

Each launch then records the answer in the session directory it just created, beside the two answers it already writes there. Leave it unset to keep `claude`'s own behaviour and be asked. [Configuration](../reference/configuration.md#keys) owns the key.
