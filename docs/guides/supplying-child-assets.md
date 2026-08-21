# Supplying your own skills, agents, and rules

Every launch gives its terminal an isolated configuration directory, and `claude` reads its user-level assets from whatever directory it was pointed at. So the skills, agents, and rules you wrote do not appear there on their own. This guide sets up the one tree that reaches every session, on every account, in every terminal.

It is a one-time step per machine. The wrapper writes no user configuration ([ADR-0015](../decisions/ADR-0015-retire-the-init-verb.md)), so nothing does this for you.

## Where the tree lives

```text
${XDG_DATA_HOME:-$HOME/.local/share}/claude-session/assets/
```

The Data base rather than State, because this is content you author and could carry to another machine. [XDG storage](../reference/xdg-storage.md#artifact-table) owns the placement.

## What it may hold

The names `claude` reads from its own configuration directory, and no others:

| Name               | Holds                                   |
| ------------------ | --------------------------------------- |
| `skills/`          | Skills, one directory each              |
| `agents/`          | Subagent definitions                    |
| `commands/`        | Single-file commands                    |
| `rules/`           | Rules that apply to every project       |
| `workflows/`       | Workflow scripts                        |
| `output-styles/`   | Output styles                           |
| `themes/`          | Colour themes                           |
| `agent-memory/`    | Per-subagent memory the child writes    |
| `CLAUDE.md`        | Personal instructions for every project |
| `keybindings.json` | Keyboard shortcuts                      |

A name the tree does not hold is not supplied, and nothing is created in its place. Start with the one or two you actually have.

The child's plugin tree is deliberately not on this list ([ADR-0106](../decisions/ADR-0106-supply-child-assets-from-one-tree.md)).

## Setting it up

Create the tree and put your assets in it:

```bash
assets="${XDG_DATA_HOME:-$HOME/.local/share}/claude-session/assets"
mkdir -p "$assets"
```

If you already keep these under `claude`'s own directory, move them:

```bash
mv ~/.claude/skills ~/.claude/agents "$assets/"
```

Moving rather than copying is the point: two trees drift, and the one under `~/.claude` is read by an unwrapped `claude` only.

If you keep them in a repository you already version, move the repository there and leave a link behind for whatever else reads it — the wrapper reads this tree and never writes it, so its contents are entirely yours to arrange.

## Checking it

```bash
claude-session doctor
```

The `session-assets-linked` check names what a launch would supply. An empty tree is a warning rather than a failure: it costs you every asset you wrote, but it never stops a launch.

After a launch, the assets appear inside the session directory as links:

```bash
ls -la "${XDG_STATE_HOME:-$HOME/.local/state}"/claude-session/accounts/*/sessions/*/*/
```

## Hooks and scripts

A hook command, a status line, or anything else your settings invoke by absolute path needs nothing here. Those are named in the settings your profile composes, and an absolute path resolves the same from any configuration directory. Only the directories `claude` discovers by name belong in the asset tree.
