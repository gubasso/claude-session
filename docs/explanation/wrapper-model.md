# The wrapper model

`claude-session` is a wrapper: its primary job is to launch another program. That makes it a different kind of CLI from one that does its own work in-process, and most of the design pressure on this codebase comes from that fact. This page explains the model. The exact flag list is in [the CLI surface](../reference/cli-surface.md); the exact resolution ladder and launch sequence are in [the process runtime](../reference/process-runtime.md); the exact codes are in [exit codes](../reference/exit-codes.md).

## The cardinal principle

Keep the wrapper's grammar small and explicit. Keep the wrapped command opaque. Do not rewrite argv without a narrow, stable reason.

Every other rule here follows from that one. A wrapper that parses its child's grammar in order to rewrite it has taken on an obligation it cannot meet: the child is developed by someone else, it gains flags on its own schedule, and each new flag is a potential breakage in the wrapper. The wrapper that survives is the one that knows almost nothing about its child.

This is the first of the project's three standing contracts, and it is why `AGENTS.md` treats a change in passthrough behaviour as a breaking change requiring a decision record.

## Three zones of argv

A wrapper invocation has three regions, and confusing them is the classic wrapper bug:

```text
claude-session-rs [WRAPPER FLAGS] <verb> [--] [CHILD ARGS...]
```

Wrapper flags come first and are a closed, documented set. They are long-form, distinctively named, and small enough to list on one screen. Anything not on that list is not a wrapper flag, no matter how much it looks like one.

The verb is either one of the wrapper's own commands or absent. When it is absent, the invocation is a passthrough and everything from the first token onward belongs to the child.

Child arguments are opaque. They are forwarded in order, byte for byte, and the wrapper forms no opinion about them.

`--` is a hard sentinel. Everything after it is child argument territory, unconditionally, even if it happens to spell a wrapper flag or a wrapper verb. There is no context in which the wrapper reinterprets a token past `--`.

## Denylist, not allowlist

The wrapper claims a denylist of flags: a short, explicit list it intercepts. Everything else forwards.

The alternative — an allowlist of child flags the wrapper understands, with anything unrecognized rejected — fails the moment the child ships a new flag. The user would then need a new wrapper release to use a feature that already works in the tool being wrapped. That is exactly the failure the passthrough contract exists to prevent.

The cost of a denylist is a genuine collision risk: if the wrapper claims a flag the child also claims, the wrapper wins and the user loses the child's version. That risk is real rather than theoretical — the child already spells `--verbose`, and it spells `-v` as its version flag.

Three things keep the cost bounded. Wrapper flags are long-form and distinctive rather than single letters. Interception is leading-position only, so a claimed spelling typed after any other token still reaches the child. And the claimed set is compared against a recorded inventory of the child's own flags, with any unnamed overlap failing the build rather than being noticed later ([ADR-0044](../decisions/ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md)). The exact spellings, their measured child status, and the escape hatches are in [the CLI surface](../reference/cli-surface.md#wrapper-owned-flags).

A collision the wrapper cannot resolve by shadowing is resolved by renaming, as `auth` was, or by composing — running the child's command inside the wrapper's own and reporting both, as `doctor` does. Nothing is left to be discovered by a user who did not know a wrapper was in the way.

## Byte-preserving argv

Arguments are forwarded verbatim, preserving order and bytes. Concretely:

- Arguments are carried as OS strings end to end, never round-tripped through UTF-8. On Unix an argument is a byte string, and a filename that is not valid UTF-8 is still a perfectly legal argument.
- An empty argument is a real argument. A shell that ran `claude-session-rs foo "" bar` passed three arguments, and the child must receive three. Filtering empties is a silent semantic change to the user's command line.
- No reordering, no deduplication, no case normalization, no quote stripping, no re-quoting.

There is no argv normalization step in this program. If a future change appears to need one, it is a change to the passthrough contract and needs a decision record before it needs code.

## Why the parser cannot do this alone

A derive-based argument parser is built to reject what it does not recognize. Configuring one to accept an unknown positional token as an external subcommand is straightforward; the parser treats an unexpected positional as a subcommand name and hands you the rest. But a leading unknown flag is not a positional. An invocation whose first token is a dash-prefixed flag the wrapper does not define is rejected as an unexpected argument before any external-subcommand handling applies. Settings that relax hyphen handling operate on a declared value, not on the top-level parse, and so do not rescue this case either.

Since a passthrough wrapper's most common invocation is exactly that shape — the user typing a child flag as the first token — the parser cannot be the only gate.

The consequence for this design: argv is split before it reaches the parser. A small, pure pre-parse scans the front of the command line, consuming only tokens the wrapper's closed flag set claims and stopping at the first token that is not one — or at `--`. If what remains begins with a wrapper verb, the parser handles it. Otherwise the remainder is child argv and never touches the parser at all.

That pre-parse is a pure function over a list of OS strings. Being pure and total makes it directly unit-testable, and it is: the golden-argv tests in [the testing strategy](./testing-strategy.md) exist precisely because this function is the single point where the passthrough contract can silently break.

## Exec, not spawn and wait

The wrapper does not stay alive beside its child. It resolves the child, prepares whatever the run selected, and then replaces its own process image with the child's ([ADR-0084](../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md)).

This is a one-shot wrapper, in the sense a shell script is one when it runs `jq` or `curl`: once the inner program is running with the right arguments and the right environment, the wrapper's job is finished. Nothing it owns needs to observe the child's exit. The account last-used marker records which account a run selected, so it is written at launch, and the log sink is flushed just before the replacement.

The payoff is that transparency stops being a set of obligations and becomes a structural fact. There is no signal forwarding to get partly wrong, no exit-status translation, no wait status to reproduce, and no second process in the tree — the child inherits the wrapper's process id, so `ps` shows one process and a signal aimed at it lands on `claude`. [ADR-0058](../decisions/ADR-0058-behave-as-stock-claude-by-default.md) generalizes the default past argv and exit status, and with supervision gone its only exception class is the wrapper's own verbs and intercepted flags.

The cost is stated once so it is not rediscovered as a bug: no wrapper work can ever run after the child. A future obligation of that shape does not get bolted on; it needs a record reversing this one, and it would bring the whole supervision problem back with it.

## The double-delivery trap this avoids

The reason to state what was avoided is that it is the trap a wrapper falls into by default, and someone will propose it again.

A surviving wrapper that spawns the child without changing process groups leaves it in the same foreground process group, where the kernel delivers every terminal-generated signal to both. So Ctrl-C already reaches the child, and a wrapper that also forwards it delivers the signal twice — a child that counts interrupts, one press to interrupt and two to quit, reads a single press as a double press. "Forward everything" is therefore a bug rather than a safe default, and the correct forwarding set is partial, which makes it a table to be maintained rather than a rule to be reasoned out.

Giving the child its own process group is worse: it stops receiving terminal signals at all, so the wrapper must forward every one and also manage which group owns the terminal. And job control needs its own mirror on top, because a stopped child under a running wrapper leaves the shell waiting on a live foreground process instead of printing its prompt.

An exec has none of these problems, because there is no second process to disagree with the first.

## Finding the child, and not finding yourself

The wrapper must locate the real `claude` binary explicitly. Relying on a bare path search is how a wrapper installed under the same name as its child ends up invoking itself, forking until something runs out.

Resolution is an explicit ladder — a configured override, then a path search — and whichever rung answers is the one that decides, including when what it named is broken. Existence and executability are checked, with distinct failures for "not found" and "found but not executable". The ladder is spelled out in [the process runtime](../reference/process-runtime.md).

Two independent guards prevent self-invocation. A marker variable is set in the child's environment, so a wrapper that finds itself as the child sees the marker and refuses. And the resolved child is compared against the wrapper's own executable by file identity, which catches the link case the marker cannot. Both, because either alone has a hole: the marker is defeated by a scrubbed environment, and the identity check is defeated by a copy rather than a link.

## Account selection and composed settings

The wrapper selects an account-wide configuration directory through `CLAUDE_CONFIG_DIR`; the child owns its saved login and other native state inside it. The wrapper never reads, copies, refreshes, fingerprints, or synchronizes that credential.

Composition leaves through one declared wrapper-added argv pair: `--settings <absolute composed-settings path>`. That pair precedes an opaque, verbatim user suffix. The wrapper preserves every user token and does not parse duplicate settings flags; see [ADR-0028](../decisions/ADR-0028-pass-composed-settings-with-the-native-flag.md).

The child keeps only the last `--settings` it is given, so a user who passes one replaces the wrapper's composed document rather than adding to it. The wrapper accepts that precedence instead of repairing it: the alternative is inspecting the suffix, which is the coupling this whole model exists to avoid. The consequence is stated where a user meets it, in [the child argument vector](../reference/process-runtime.md#child-argument-vector).

Account selection and profile selection are separate axes, explained in [session isolation](./session-isolation.md).

The child's environment is otherwise inherited, with the wrapper's own `CLAUDE_SESSION_*` variables scrubbed out and the recursion marker then set back. A child should never be able to observe the wrapper's internal state by reading its environment, both because it is none of the child's business and because a nested invocation would inherit stale values — and the marker is the deliberate exception, because a nested invocation seeing it is exactly how the guard fires.

## The proxy seam

Some users front `claude` with a local proxy — for token compression, request logging, or routing. The child already supports this through an environment variable naming its API base URL.

The wrapper's contribution is a seam, not a feature — and the seam is inheritance. The wrapper does not touch that variable, so exporting it is all a user has to do; there is nothing to compose and no injection surface to maintain. `claude-session` implements no proxying, no compression, and no request rewriting of its own, and it should not grow any.

The boundary is worth stating plainly because it is the kind of thing that erodes. Every request-manipulating feature added inside the wrapper is a feature that must track the upstream API, duplicate an existing external tool, and be debugged inside a process whose job is to launch another process. The seam stays a seam.

## Further reading

- [Command Line Interface Guidelines](https://clig.dev/)
- [Beyond Ctrl-C: the dark corners of Unix signal handling](https://sunshowers.io/posts/beyond-ctrl-c-signals/)
- [`execve(2)`](https://man.archlinux.org/man/execve.2)
