# The wrapper model

`claude-session` is a wrapper: its primary job is to launch another program. That makes it a different kind of CLI from one that does its own work in-process, and most of the design pressure on this codebase comes from that fact. This page explains the model. The exact flag list is in [the CLI surface](../reference/cli-surface.md); the exact signal and resolution tables are in [the process runtime](../reference/process-runtime.md); the exact codes are in [exit codes](../reference/exit-codes.md).

## The cardinal principle

**Keep the wrapper's grammar small and explicit. Keep the wrapped command opaque. Do not rewrite argv without a narrow, stable reason.**

Every other rule here follows from that one. A wrapper that parses its child's grammar in order to rewrite it has taken on an obligation it cannot meet: the child is developed by someone else, it gains flags on its own schedule, and each new flag is a potential breakage in the wrapper. The wrapper that survives is the one that knows almost nothing about its child.

This is the first of the project's three standing contracts, and it is why `AGENTS.md` treats a change in passthrough behaviour as a breaking change requiring a decision record.

## Three zones of argv

A wrapper invocation has three regions, and confusing them is the classic wrapper bug:

```text
claude-session [WRAPPER FLAGS] <verb> [--] [CHILD ARGS...]
```

**Wrapper flags** come first and are a closed, documented set. They are long-form, distinctively named, and small enough to list on one screen. Anything not on that list is not a wrapper flag, no matter how much it looks like one.

**The verb** is either one of the wrapper's own commands or absent. When it is absent, the invocation is a passthrough and everything from the first token onward belongs to the child.

**Child arguments** are opaque. They are forwarded in order, byte for byte, and the wrapper forms no opinion about them.

`--` is a hard sentinel. Everything after it is child argument territory, unconditionally, even if it happens to spell a wrapper flag or a wrapper verb. There is no context in which the wrapper reinterprets a token past `--`.

## Denylist, not allowlist

The wrapper claims a **denylist** of flags: a short, explicit list it intercepts. Everything else forwards.

The alternative — an allowlist of child flags the wrapper understands, with anything unrecognized rejected — fails the moment the child ships a new flag. The user would then need a new wrapper release to use a feature that already works in the tool being wrapped. That is exactly the failure the passthrough contract exists to prevent.

The cost of a denylist is a genuine collision risk: if the wrapper claims a short flag and the child later claims the same one, the wrapper wins and the user loses access to the child's version. This is why wrapper flags are long-form and distinctive rather than single letters, and why every claimed flag is listed with a reason in [the CLI surface](../reference/cli-surface.md).

## Byte-preserving argv

Arguments are forwarded **verbatim**, preserving order and bytes. Concretely:

- Arguments are carried as OS strings end to end, never round-tripped through UTF-8. On Unix an argument is a byte string, and a filename that is not valid UTF-8 is still a perfectly legal argument.
- An **empty argument is a real argument**. A shell that ran `claude-session foo "" bar` passed three arguments, and the child must receive three. Filtering empties is a silent semantic change to the user's command line.
- No reordering, no deduplication, no case normalization, no quote stripping, no re-quoting.

There is no argv normalization step in this program. If a future change appears to need one, it is a change to the passthrough contract and needs a decision record before it needs code.

## Why the parser cannot do this alone

A derive-based argument parser is built to reject what it does not recognize. Configuring one to accept an unknown _positional_ token as an external subcommand is straightforward; the parser treats an unexpected positional as a subcommand name and hands you the rest. But a leading unknown **flag** is not a positional. An invocation whose first token is a dash-prefixed flag the wrapper does not define is rejected as an unexpected argument before any external-subcommand handling applies. Settings that relax hyphen handling operate on a declared value, not on the top-level parse, and so do not rescue this case either.

Since a passthrough wrapper's most common invocation is exactly that shape — the user typing a child flag as the first token — the parser cannot be the only gate.

The consequence for this design: **argv is split before it reaches the parser.** A small, pure pre-parse scans the front of the command line, consuming only tokens the wrapper's closed flag set claims and stopping at the first token that is not one — or at `--`. If what remains begins with a wrapper verb, the parser handles it. Otherwise the remainder is child argv and never touches the parser at all.

That pre-parse is a pure function over a list of OS strings. Being pure and total makes it directly unit-testable, and it is: the golden-argv tests in [the testing strategy](./testing-strategy.md) exist precisely because this function is the single point where the passthrough contract can silently break.

## Spawn and wait, not exec

Replacing the wrapper's own process image with the child's is the cheapest way to be transparent — no signal forwarding, no exit-status translation, no extra process in the tree. This project does not do it.

The reason is post-flight work. After the child exits, the wrapper must sync credential state and project-trust state back out of the isolated session directory. `exec` never returns, so that work would never happen. Correctness beats elegance here, and the cost is a real one: choosing to stay alive means owning signal forwarding, terminal semantics, and exit-status fidelity by hand.

The wrapper's obligation, having made that choice, is to be **behaviourally indistinguishable** from `exec` in everything the user can observe: the same exit status, the same terminal behaviour, the same response to Ctrl-C.

## Signals and the double-delivery trap

The naive design — catch every terminal signal and forward it to the child — is wrong, and wrong in a way that is easy to miss in testing.

When the wrapper spawns the child without changing process groups, the child stays in the wrapper's **foreground process group**. A terminal-generated signal is delivered by the kernel to _every_ process in that group. So Ctrl-C already reaches the child. A wrapper that also forwards it delivers the signal twice, and a child that counts interrupts — one press to interrupt the current operation, two to quit — sees a single press as a double press.

There are two coherent topologies, and the choice must be made once, deliberately:

- **Share the group.** Terminal signals reach the child directly. The wrapper forwards only the signals the terminal does _not_ broadcast, and otherwise stays out of the way. Simple, and correct for interactive use.
- **Give the child its own group.** The child no longer receives terminal signals at all, so the wrapper must forward every one of them, and must also manage which group owns the terminal so the child can still read from it.

This project takes the first: the child shares the wrapper's foreground process group, and forwarding is deliberately partial. Which signals are forwarded, and which are left to the kernel, is the matrix in [the process runtime](../reference/process-runtime.md). The design consequence worth stating here is that "forward everything" is a bug, not a safe default.

The stop-and-continue signals need one further note. When the child is stopped and the wrapper is not, the shell sees a live foreground process and does not print its prompt. The wrapper must stop itself too — by re-raising the signal on itself once the child has stopped — so that the job-control illusion holds.

## Finding the child, and not finding yourself

The wrapper must locate the real `claude` binary explicitly. Relying on a bare path search is how a wrapper installed under the same name as its child ends up invoking itself, forking until something runs out.

Resolution is an explicit ladder — an environment override, then configuration, then a path search, then a bundled fallback — and it is checked at each step for existence and executability, with distinct failures for "not found" and "found but not executable". The ladder is spelled out in [the process runtime](../reference/process-runtime.md).

Two independent guards prevent self-invocation. A marker variable is set in the child's environment, so a wrapper that finds itself as the child sees the marker and refuses. And the resolved path is canonicalized and compared against the wrapper's own executable, which catches the symlink case the marker cannot. Both, because either alone has a hole: the marker is defeated by a scrubbed environment, and the path check is defeated by a copy rather than a link.

## Isolation by environment injection

The wrapper does not modify the child's configuration files. It points the child at a different configuration directory entirely, by setting the child's configuration-directory environment variable to a per-session path that the wrapper owns.

This is the whole isolation mechanism, and its virtue is that it needs no cooperation from the child beyond a variable the child already honours. The reasoning about _which_ session a given terminal gets, and why that directory is durable state rather than cache, is in [session isolation](./session-isolation.md).

The child's environment is otherwise inherited, with the wrapper's own internal variables scrubbed out. A child should never be able to observe the wrapper's internal state by reading its environment, both because it is none of the child's business and because a nested invocation would inherit stale values.

## The proxy seam

Some users front `claude` with a local proxy — for token compression, request logging, or routing. The child already supports this through an environment variable naming its API base URL.

The wrapper's contribution is a **seam, not a feature**: the child's environment is composable, so that base URL and any other variables can be injected from configuration or the command line. `claude-session` implements no proxying, no compression, and no request rewriting of its own, and it should not grow any.

The boundary is worth stating plainly because it is the kind of thing that erodes. Every request-manipulating feature added inside the wrapper is a feature that must track the upstream API, duplicate an existing external tool, and be debugged inside a process whose job is to launch another process. The seam stays a seam.

## Further reading

- [Command Line Interface Guidelines](https://clig.dev/)
- [Beyond Ctrl-C: the dark corners of Unix signal handling](https://sunshowers.io/posts/beyond-ctrl-c-signals/)
- [Signal handling — Command Line Applications in Rust](https://rust-cli.github.io/book/in-depth/signals.html)
- [`signal-hook`](https://docs.rs/signal-hook/)
