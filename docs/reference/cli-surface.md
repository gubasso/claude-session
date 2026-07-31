# CLI surface

The wrapper's own grammar: what `claude-session` claims, what it forwards, and the parser shape that makes verbatim passthrough work. For the reasoning behind these rules, see [the wrapper model](../explanation/wrapper-model.md).

This describes normative design. The crate is pre-implementation.

## Invocation shape

```text
claude-session [WRAPPER FLAGS] <verb> [VERB ARGS...]
claude-session [WRAPPER FLAGS] [--] [CHILD ARGS...]
```

Wrapper flags come **before** the verb. There is no wrapper flag valid after the verb, and no wrapper flag valid after `--`.

When the first non-flag token is a wrapper verb, the invocation is a wrapper command. Otherwise the invocation is a passthrough and every remaining token belongs to the child. An invocation with no tokens at all is a passthrough with no arguments.

## Wrapper-owned flags

This table is the denylist. Every flag on it is intercepted by the wrapper **in leading position** and does not reach the child there. **Every flag not on it is forwarded verbatim**, whether or not the wrapper recognizes it, and whether or not it exists in the child.

The child-status column is measured, not assumed. It is the intersection audited by [ADR-0044](../decisions/ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md), taken from `claude` 2.1.220 on 2026-07-31; the `child-flag-and-verb-inventory` fact in [research tracking](./research-tracking.yaml) owns its freshness.

| Flag               | Meaning                                              | Why the wrapper claims it                                                                                                   | Child status                                 |
| ------------------ | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
| `--verbose`        | Increase diagnostic verbosity; repeatable            | The wrapper's own diagnostics need a control separate from the child's                                                      | Collides: the child has `--verbose` too      |
| `--quiet`, `-q`    | Suppress all diagnostics below error                 | Pairs with `--verbose`; required for scripted use                                                                           | Free                                         |
| `--config <path>`  | Override the wrapper's own configuration file        | Needed before configuration is loaded, so it cannot itself come from configuration                                          | Free                                         |
| `--account <name>` | Select the account and stored authentication context | The wrapper owns account selection; the child owns its credential                                                           | Free                                         |
| `--session <id>`   | Override the derived session group identity          | The wrapper owns session identity; see [session isolation](../explanation/session-isolation.md)                             | Free; the child's own flag is `--session-id` |
| `--profile <name>` | Select the settings profile to compose               | The wrapper owns composition; declared on the launch and on `config` only, per [configuration](./configuration.md#commands) | Free                                         |
| `--dry-run`        | Resolve and report what would happen; spawn nothing  | A wrapper-level rehearsal has no child equivalent                                                                           | Free                                         |
| `--version`, `-V`  | Print the wrapper's version and the resolved child's | Must report both, which the child cannot do                                                                                 | `--version` collides by design; `-V` free    |
| `--help`, `-h`     | Print the wrapper's help                             | Must describe the wrapper's grammar, not the child's                                                                        | Collides by design                           |

Three properties of this table are contractual:

**Long-form and distinctive.** Short forms are used only where the convention is universal (`-q`, `-V`, `-h`). Claiming a short flag that the child later wants is a collision the wrapper wins and the user loses, so the set stays small. `-v` is deliberately absent: the child spells it `--version`, so claiming it for verbosity would change a token's meaning rather than shadow it ([ADR-0044](../decisions/ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md)).

**Append-only in spirit.** Adding a flag to this table removes a flag from the child's reachable surface. That is a passthrough-contract change, and it requires a decision record — see [ADR-0002](../decisions/ADR-0002-verbatim-argv-passthrough.md) and [ADR-0003](../decisions/ADR-0003-reserve-a-small-wrapper-cli-surface.md).

**Audited, not asserted.** An intersection between this table and the child's inventory that is not named in the child-status column fails the build. The mechanism is the collision-audit test in [testing and quality](./testing-and-quality.md#mandatory-tests).

### Flag spelling

Recognition is exact. A token is a wrapper flag only when all of these hold; anything else is child argv ([ADR-0043](../decisions/ADR-0043-match-wrapper-flags-by-exact-leading-spelling.md)).

| Rule                 | Consequence                                                                                                     |
| -------------------- | --------------------------------------------------------------------------------------------------------------- |
| Byte-exact           | `--config` is claimed; `--Config`, `--configg`, and `--conf` are not. Matching is ASCII case-sensitive.         |
| No abbreviation      | A unique prefix is not the flag. `--acc` reaches the child.                                                     |
| No bundling          | Short flags match only as whole single-letter tokens. `-qh` and `-vv` are child argv, as is any longer cluster. |
| Leading position     | Recognition stops at the first token that is not a wrapper flag, and at `--`.                                   |
| Value attachment     | A value-taking flag accepts `--flag=<value>` and `--flag <value>`.                                              |
| Values are not flags | In the separated form a next token beginning with `-` is not consumed as the value.                             |

Documentation and examples use the `--flag=<value>` form, because it is unambiguous at a glance about which side of the boundary the value belongs to.

Repetition: `--verbose` is repeatable by repeating the whole spelling, and `--quiet` is idempotent. Every other wrapper flag is accepted at most once, and a repeat is a `Usage` error. What each verbosity level means, and why `--verbose` with `--quiet` is rejected, is in [logging and output](./logging-and-output.md#verbosity).

### Machine output is not on this table

`--json` is **verb-level**, accepted after the verb, and every verb that produces data owns its own:

```text
claude-session account list --json
```

A global `--format` would sit on the denylist above and cost the child a flag permanently, in exchange for nothing — machine output has no meaning for a passthrough invocation, which never emits wrapper output at all. Owning the flag per verb also keeps each verb's output schema independent, so one verb's document can change shape without implying anything about another's. See [ADR-0024](../decisions/ADR-0024-machine-output-is-a-per-verb-flag.md); the format contract itself is in [logging and output](./logging-and-output.md#machine-output).

### When the child owns the same name

The wrapper wins in leading position, deterministically and **silently**. It does not warn, because warning would require the model of the child's grammar [ADR-0002](../decisions/ADR-0002-verbatim-argv-passthrough.md) forbids.

Three things reach the child's own spelling:

| Escape               | Example                          | What the child receives                                 |
| -------------------- | -------------------------------- | ------------------------------------------------------- |
| The sentinel         | `claude-session -- --verbose`    | `--verbose`                                             |
| Any earlier token    | `claude-session -p hi --verbose` | `-p hi --verbose` — recognition already stopped at `-p` |
| A different spelling | `claude-session --session-id X`  | `--session-id X` — a near-miss is not claimed           |

`--` is unconditional. Everything after it is child argument territory even if it spells a wrapper flag or verb, and **a second `--` after the boundary is an ordinary child argument** — the wrapper consumes the first and never inspects, strips, or counts the rest.

Discovering a new collision is a procedure, not a note. Rebuild the child's inventory as the `child-flag-and-verb-inventory` fact in [research tracking](./research-tracking.yaml) describes, diff it against the child-status column, and resolve every difference before release: a shadowing collision is recorded in the column, and one that would change a token's meaning forces the wrapper's spelling to be dropped or renamed under a new decision record.

## Wrapper verbs

Verbs are top-level rather than nested under a namespace verb. Nesting would add a token to every wrapper invocation to solve a collision problem that the closed, documented verb list already solves.

| Verb         | Purpose                                                                   | Grammar specified in                          |
| ------------ | ------------------------------------------------------------------------- | --------------------------------------------- |
| `account`    | Manage accounts: login, list, status, remove                              | [accounts](./accounts.md)                     |
| `config`     | Resolve, validate, and report the wrapper's configuration; no subcommands | [configuration](./configuration.md#commands)  |
| `profile`    | List the available settings profiles; no subcommands                      | [configuration](./configuration.md#commands)  |
| `doctor`     | Diagnose every subsystem, then run the child's own `doctor`               | [logging and output](./logging-and-output.md) |
| `completion` | Emit shell completions for the wrapper's grammar                          | [Help](#help)                                 |
| `man`        | Emit man pages generated from the wrapper's grammar                       | [Help](#help)                                 |
| `version`    | Print the wrapper's version and the resolved child's path and version     | [Version output](#version-output)             |
| `help`       | Print the wrapper's help, or one verb's                                   | [Help](#help)                                 |

Two verb names overlap the child's, measured against `claude` 2.1.220 on 2026-07-31, and each resolves differently:

| Child verb | Resolution | Reason                                                                                                                                                                                                                   |
| ---------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `auth`     | Renamed    | Native auth is the child's own credential flow and stays reachable as passthrough; `account` is the wrapper's namespace ([ADR-0030](../decisions/ADR-0030-use-account-login-for-wrapper-authentication.md)).             |
| `doctor`   | Composed   | The two reports answer different questions, so the wrapper runs its own checks and then the child's, passing that output through unmodified ([ADR-0045](../decisions/ADR-0045-compose-doctor-with-the-child-report.md)). |

Those are the only two resolutions available. A wrapper verb may keep a name the child owns **only** when it runs the child's command as part of its own and reports the result; otherwise it is renamed. Shadowing a child verb and leaving `--` as the sole remedy is not one of them, because a user reaching for a diagnostic does not know a wrapper is in the way. Every overlap is named in this table with its reasoning, and the audit that keeps the table honest is the same one that covers flags ([ADR-0044](../decisions/ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md)).

There is no `init`. Configuration is optional — every key has a compiled-in default — and the wrapper never writes the user's configuration, so there is no scaffold to create. Users copy a [generated example](./configuration.md#generated-examples-and-schema) instead. See [ADR-0015](../decisions/ADR-0015-retire-the-init-verb.md).

## Passthrough contract

Forwarding is **verbatim**. Specifically:

| Property        | Rule                                                                                                                                            |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Order           | Preserved exactly. No reordering, no sorting, no moving flags before positionals.                                                               |
| Bytes           | Preserved exactly. Arguments are carried as OS strings and never round-tripped through UTF-8. A non-UTF-8 argument reaches the child unchanged. |
| Empty arguments | Preserved. An empty string is a real argument and is never filtered.                                                                            |
| Count           | Preserved. No deduplication, no splitting on whitespace, no joining.                                                                            |
| Quoting         | Untouched. The shell already removed quotes; the wrapper does not re-add or re-interpret them.                                                  |
| Unknown flags   | Forwarded. An unrecognized flag is a child flag by definition.                                                                                  |

**There is no argv normalization step.** A change that adds one is a change to the passthrough contract and requires a decision record before it requires code.

Standard input, standard output, and standard error are inherited by the child unmodified. The wrapper writes nothing to standard output during a passthrough invocation; see [logging and output](./logging-and-output.md).

For an account-backed group, [ADR-0028](../decisions/ADR-0028-pass-composed-settings-with-the-native-flag.md) narrowly authorizes one wrapper-owned prefix, `--settings <absolute group settings path>`. Every user-supplied token remains an untouched suffix with order, bytes, count, and `--` sentinel preserved. The wrapper does not parse or normalize that suffix.

## Parser shape

A derive-based parser cannot be the only gate, and the reason is specific.

Configuring a parser to accept unknown **external subcommands** makes it treat an unexpected _positional_ token as a subcommand name. A leading unknown **flag** is not a positional: `claude-session --print hello` is rejected as an unexpected argument before external-subcommand handling applies. Relaxed hyphen handling does not rescue this, because it applies to a declared value rather than to the top-level parse. Since a leading child flag is one of the most common passthrough invocations, the parser must not see it.

The contract is therefore:

1. **Pre-split argv.** A pure function scans the front of the argument vector, consuming only tokens the wrapper's denylist claims and their values. It stops at the first token that is not a wrapper flag, or at `--`.
2. **Classify.** If the next token is a wrapper verb, the remainder goes to the parser. Otherwise the remainder — including `--` handling — is child argv and is never handed to the parser.
3. **Parse only the wrapper's part.** The parser sees a grammar in which every token is one it defines.

The pre-split is pure and total over a list of OS strings, which is what makes it directly unit-testable. It is the single point where the passthrough contract can silently break, and the golden-argv tests in [testing and quality](./testing-and-quality.md) exist to guard it.

**The split itself never fails.** It classifies; only the wrapper's own parse and validation step exits. That keeps totality a property of the function rather than a claim about its callers, and it gives exactly three outcomes for a token that looks like a wrapper flag:

| Token                                                                                   | Outcome                                                                           |
| --------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Not an exact claimed spelling — `--configg`, `--acc`, `--CONFIG`, `-vq`                 | Split stops. The token and everything after it is child argv, forwarded verbatim. |
| Claimed, value missing — `--config` last, or before `--`, or before a `-`-leading token | `Usage`, naming the flag on standard error.                                       |
| Claimed, value malformed — `--config=`, `--verbose=1`                                   | `Usage`, naming the flag on standard error.                                       |

The wrapper never suggests that an unrecognized token was a mistyped wrapper flag. Suggestion machinery needs a model of the child's flags in order to know what it is _not_ looking at, and the wrapper does not have one. Codes are in [exit codes](./exit-codes.md#wrapper-matrix).

Two parser settings are hazards rather than tools here. Long-argument inference is opt-in and stays off, since a unique prefix today is not a unique prefix after the child ships a flag. A trailing variable-argument declaration is not a substitute for the pre-split either, and it carries a live argv-corruption risk: a value delimiter configured on that argument still applies, which would split a child token on a character the user typed literally.

The parser is additionally configured to disable its automatic version flag, so that `--version` reports both the wrapper and the resolved child rather than the wrapper alone.

## Help

`--help` output is **generated by the parser**. A hand-maintained flag table is forbidden: it drifts from the parser the first time a flag is added and the table is not.

Authored prose the parser cannot generate — worked passthrough examples, the `--` explanation, a pointer to this documentation — lives in a text file under the output module and is included into the parser's long help at compile time.

Help describes the **wrapper's** grammar only. It does not reproduce, summarize, or link into the child's flag list, because that list is not the wrapper's to track. It should say, once and plainly, that unrecognized arguments are forwarded.

The `help` verb is the same surface under another spelling: `claude-session help [<verb>]` prints exactly what `--help` and `<verb> --help` print. Requested help is a **result** — standard output, exit `0`. Help printed because an invocation was malformed is a **diagnostic** — standard error, exit `Usage`. The parser's own default differs on both counts and is overridden; see [exit codes](./exit-codes.md#wrapper-matrix).

Shell completions cover the wrapper's grammar for the same reason. Completions never attempt to complete child arguments.

```text
claude-session completion <bash|elvish|fish|powershell|zsh>
```

The five are the full set the generator supports, so the list is the dependency's rather than a subset this project would have to justify and revisit. The script is the verb's result and is written raw to standard output: no header, no summary, no diagnostic. An unrecognized shell exits `Usage`.

**Man pages are generated from the same parser tree.** Because help, completions, and man pages all read one `Command` tree, the flag list has a single source and no surface can drift from another. The authored prose file included into long help is included into the man page too. See [ADR-0016](../decisions/ADR-0016-ship-man-pages.md).

```text
claude-session man [--out-dir <dir>]
```

Without `--out-dir`, `man` writes roff to standard output. With it, the page set is written into that directory — one page for the wrapper and one per verb — and standard output stays empty, so a packager can render at build time while a user previews without installing anything. Neither form takes `--json`: roff is not data.

## Version output

`--version` and the `version` verb print the same two lines on standard output:

```text
claude-session 0.1.0
claude /usr/local/bin/claude 1.0.2
```

Each line is `<name> [<path>] <version>`, with the version last so a caller can take the final field. Reporting both is the point: a user debugging wrapper behaviour needs to know which child was actually found, and path resolution is the most common source of surprise.

When the child cannot be resolved or its version cannot be read, the second line names that condition in place of the version and **the exit stays `0`**. The wrapper's version is a fact it always knows; refusing to report it because the child is missing would withhold the one answer the user came for. `doctor` is where a missing child fails.

The verb also takes `--json`, and the flag form does not — `--version` is intercepted before the verb split, where no verb-level flag applies:

```json
{
  "version": "0.1.0",
  "child": { "path": "/usr/local/bin/claude", "version": "1.0.2", "status": "ok" }
}
```

`child.status` is one of `ok`, `not-found`, `not-executable`, or `unparsable`; `child.version` is omitted unless the status is `ok`, and `child.path` unless resolution got far enough to name one. The document carries no `schema_version`, per [machine output](./logging-and-output.md#machine-output).

## Confirmation and non-interactive use

Two verbs need a person present. No others do.

| Verb             | Why a person is needed                                                       | Escape when there is no terminal                          |
| ---------------- | ---------------------------------------------------------------------------- | --------------------------------------------------------- |
| `account remove` | It removes local authentication and child state                              | `--yes`                                                   |
| `account login`  | Native login uses the child's interactive flow; token paste reads a terminal | Long-lived subscription-token mode with `--token --stdin` |

Every other verb — `config`, `profile`, `doctor`, `completion`, `man`, `version`, `help` — is read-only or inert. There is nothing to agree to, so none of them prompts and none of them gates.

**Without a terminal, a confirming verb fails rather than prompting or proceeding.** When no controlling terminal is available and no escape was given, the verb stops **before any side effect** and exits `Unavailable` (69). The diagnostic names the escape above; token ingestion through `--stdin` follows [ADR-0027](../decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md).

Reading the absence of a terminal as consent is the alternative, and it makes `account remove` silent under a pipe. Prompting anyway is worse: the process hangs on a stream nobody is reading. See [ADR-0021](../decisions/ADR-0021-fail-closed-without-a-terminal.md).

### Why `--yes` is not in the flag table

`--yes` is a **verb-level** flag, accepted after the verb:

```text
claude-session account remove work --yes
```

Its absence from the wrapper-owned flag table above is the design, not an oversight. A top-level flag is intercepted before the passthrough split and is therefore subtracted from the child's reachable surface for good; a flag appearing after a wrapper verb is parsed inside an invocation the child never sees, so it costs the child nothing. `--json` and `doctor --list` are verb-level for the same reason.

There is no `--non-interactive`. Detecting the missing terminal already produces exactly that behaviour, so a flag requesting it would be surface bought for nothing.

**Passthrough is untouched.** The wrapper does not inspect standard input on a passthrough invocation, and the child's own prompting is the child's business.

## Exit behaviour

Wrapper verbs exit with codes from the wrapper's matrix. A passthrough invocation exits with the child's status. The two regimes and the boundary between them are in [exit codes](./exit-codes.md).

## Further reading

- [clap `Command` documentation](https://docs.rs/clap/latest/clap/struct.Command.html)
- [Command Line Interface Guidelines](https://clig.dev/)
