# CLI surface

The wrapper's own grammar: what `claude-session` claims, what it forwards, and the parser shape that makes verbatim passthrough work. For the reasoning behind these rules, see [the wrapper model](../explanation/wrapper-model.md).

The passthrough, `help`, `version`, `doctor`, the whole `account` namespace — `login [name]` with `--token`, `--stdin`, and `--minted-at`, plus `list`, `status`, and `remove` with `--yes` — `completion <shell>`, `man`, `profile`, and `config` are implemented, and so is requested help for every verb that answers one on its own — `account --help`, `account <subcommand> --help`, `completion --help`, `man --help`, `profile --help`, `config --help`, `doctor --help`, `version --help`, and the matching `help <verb>` spellings. The `help` verb has no requested help of its own, because a reader asking for it is already reading the composed surface it would describe.

## Invocation shape

```text
claude-session-rs [WRAPPER FLAGS] <verb> [VERB ARGS...]
claude-session-rs [WRAPPER FLAGS] [--] [CHILD ARGS...]
```

Wrapper flags come before the verb. There is no wrapper flag valid after the verb, and no wrapper flag valid after `--`.

When the first non-flag token is a wrapper verb, the invocation is a wrapper command. Otherwise the invocation is a passthrough and every remaining token belongs to the child. An invocation with no tokens at all is syntactically a passthrough with no arguments; any passthrough still requires both account and profile selections to resolve before launch.

## Wrapper-owned flags

This table is the denylist. Every flag on it is intercepted by the wrapper in leading position and does not reach the child there. Every flag not on it is forwarded verbatim, whether or not the wrapper recognizes it, and whether or not it exists in the child.

The child-status column is measured, not assumed. It is the intersection audited by [ADR-0044](../decisions/ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md), taken from `claude` 2.1.220 on 2026-08-06 and recorded in [the child inventory](../../tests/fixtures/child-inventory.yaml); the `child-flag-and-verb-inventory` fact in [research tracking](./research-tracking.yaml) owns its freshness.

| Flag               | Meaning                                              | Why the wrapper claims it                                                                                                   | Child status                              |
| ------------------ | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| `--verbose`        | Increase diagnostic verbosity; repeatable            | The wrapper's own diagnostics need a control separate from the child's                                                      | Collides: the child has `--verbose` too   |
| `--quiet`, `-q`    | Suppress all diagnostics below error                 | Pairs with `--verbose`; required for scripted use                                                                           | Free                                      |
| `--config <path>`  | Override the wrapper's own configuration file        | Needed before configuration is loaded, so it cannot itself come from configuration                                          | Free                                      |
| `--account <name>` | Select the account and stored authentication context | The wrapper owns account selection; the child owns its credential                                                           | Free                                      |
| `--profile <name>` | Select the settings profile to compose               | The wrapper owns composition; declared on the launch and on `config` only, per [configuration](./configuration.md#commands) | Free                                      |
| `--version`, `-V`  | Print the wrapper's version and the resolved child's | Must report both, which the child cannot do                                                                                 | `--version` collides by design; `-V` free |
| `--help`, `-h`     | Print the wrapper's help, then the child's           | Must describe the wrapper's grammar, which the child cannot; composes so the child's is not lost                            | Collides by design                        |

Three properties of this table are contractual:

Long-form and distinctive. Short forms are used only where the convention is universal (`-q`, `-V`, `-h`). Claiming a short flag that the child later wants is a collision the wrapper wins and the user loses, so the set stays small. `-v` is deliberately absent: the child spells it `--version`, so claiming it for verbosity would change a token's meaning rather than shadow it ([ADR-0044](../decisions/ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md)).

Append-only in spirit. Adding a flag to this table removes a flag from the child's reachable surface. That is a passthrough-contract change, and it requires a decision record — see [ADR-0002](../decisions/ADR-0002-verbatim-argv-passthrough.md) and [ADR-0003](../decisions/ADR-0003-reserve-a-small-wrapper-cli-surface.md). Removal takes a record for the same reason, in the other direction: [ADR-0072](../decisions/ADR-0072-retire-the-dry-run-flag.md) retired `--dry-run` and returned that spelling to the child.

Every row has a contract. A flag appears here only once its behaviour, its output, and its failures are specified on some page. A claimed spelling with nothing behind it costs the child a token in exchange for nothing, and makes the denylist-membership test assert a row that means nothing.

Audited, not asserted. An intersection between this table and the child's inventory that is not named in the child-status column fails the build. The mechanism is the collision audit in `tests/collision_audit/`, which reads this table and [the child inventory](../../tests/fixtures/child-inventory.yaml) and compares them; see [testing and quality](./testing-and-quality.md#mandatory-tests).

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

Value types split on whether the value is a path or a name. `--config` takes a path, so its value is an OS string and bytes that are not valid UTF-8 are accepted — a Unix path is a byte string. `--account` and `--profile` name things the wrapper constructs directory components from and prints into `--json` documents and log records, so each requires valid UTF-8 and a value that is not exits `Usage`. Each must also satisfy [the identifier rules](./xdg-storage.md#identifiers); a value that does not exits `Usage`.

### Machine output is not on this table

`--json` is verb-level, accepted after the verb, and every verb that produces data owns its own:

```text
claude-session-rs account list --json
```

A global `--format` would sit on the denylist above and cost the child a flag permanently, in exchange for nothing — machine output has no meaning for a passthrough invocation, which never emits wrapper output at all. Owning the flag per verb also keeps each verb's output schema independent, so one verb's document can change shape without implying anything about another's. See [ADR-0024](../decisions/ADR-0024-machine-output-is-a-per-verb-flag.md); the format contract itself is in [logging and output](./logging-and-output.md#machine-output).

### When the child owns the same name

The wrapper wins in leading position, deterministically and silently. It does not warn, because warning would require the model of the child's grammar [ADR-0002](../decisions/ADR-0002-verbatim-argv-passthrough.md) forbids.

Winning is conditional, and one test decides it for flags and verbs alike ([ADR-0079](../decisions/ADR-0079-compose-every-overlapping-surface-with-the-child.md)). A claimed spelling the child also owns is kept only when the shared surface is read-only — checks, reports, help, version — and then it composes: the wrapper's own output first, then the child's under [the composed-output delimiter](./logging-and-output.md#composed-output). A shared surface that runs the agent, opens a terminal interface, or changes state is renamed instead, because composing it would perform the effect twice and shadowing it would hide the child's behaviour behind the wrapper's.

Whether the child owns a name is measured before it is claimed, not assumed from its documentation: the inventory is rebuilt as the procedure below describes, and a claimed spelling with no counterpart in it composes nothing. Every other token — every flag, verb, and argument the wrapper does not claim — reaches the child untouched, and the wrapper does nothing else with it.

Three things reach the child's own spelling:

| Escape               | Example                             | What the child receives                                 |
| -------------------- | ----------------------------------- | ------------------------------------------------------- |
| The sentinel         | `claude-session-rs -- --verbose`    | `--verbose`                                             |
| Any earlier token    | `claude-session-rs -p hi --verbose` | `-p hi --verbose` — recognition already stopped at `-p` |
| A different spelling | `claude-session-rs --account-id X`  | `--account-id X` — a near-miss is not claimed           |

`--` is unconditional. Everything after it is child argument territory even if it spells a wrapper flag or verb, and a second `--` after the boundary is an ordinary child argument — the wrapper consumes the first and never inspects, strips, or counts the rest.

Discovering a new collision is a procedure, not a note. Rebuild the child's inventory as the `child-flag-and-verb-inventory` fact in [research tracking](./research-tracking.yaml) describes, diff it against the child-status column, and resolve every difference before release: a shadowing collision is recorded in the column, and one that would change a token's meaning forces the wrapper's spelling to be dropped or renamed under a new decision record.

## Wrapper verbs

Verbs are top-level rather than nested under a namespace verb. Nesting would add a token to every wrapper invocation to solve a collision problem that the closed, documented verb list already solves.

| Verb         | Purpose                                                                   | Grammar specified in                         |
| ------------ | ------------------------------------------------------------------------- | -------------------------------------------- |
| `account`    | Manage accounts: login, bind, list, status, remove                        | [accounts](./accounts.md)                    |
| `config`     | Resolve, validate, and report the wrapper's configuration; no subcommands | [configuration](./configuration.md#commands) |
| `profile`    | List the available settings profiles; no subcommands                      | [configuration](./configuration.md#commands) |
| `doctor`     | Diagnose every subsystem, then run the child's own `doctor`               | [doctor](./doctor.md)                        |
| `completion` | Emit shell completions for the wrapper's grammar                          | [Help](#help)                                |
| `man`        | Emit man pages generated from the wrapper's grammar                       | [Help](#help)                                |
| `version`    | Print the wrapper's version and the resolved child's path and version     | [Version output](#version-output)            |
| `help`       | Print the wrapper's help, or one verb's                                   | [Help](#help)                                |

### Doctor flags

These spellings are scoped to `doctor` and were measured against the child `doctor` surface recorded in the checked-in inventory.

| Flag           | Meaning                                              | Child status       |
| -------------- | ---------------------------------------------------- | ------------------ |
| `--help`, `-h` | Print this verb's help, then the child's             | Collides by design |
| `--json`       | Emit one versioned JSON report or list               | Free               |
| `--list`       | Project catalog metadata without any probes          | Free               |
| `--strict`     | Promote a warning to exit one when otherwise healthy | Free               |

Two verb names overlap the child's, measured against `claude` 2.1.220 on 2026-08-06, and the read-only test in [when the child owns the same name](#when-the-child-owns-the-same-name) resolves each:

| Child verb | Shared surface                              | Resolution | Reason                                                                                                                                                                                                                                                                           |
| ---------- | ------------------------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `auth`     | A credential flow that changes stored state | Renamed    | Running it twice would mint two credentials, and its in-terminal `/login` cannot be intercepted at all; native auth stays reachable as passthrough and `account` is the wrapper's namespace ([ADR-0030](../decisions/ADR-0030-use-account-login-for-wrapper-authentication.md)). |
| `doctor`   | A diagnostic report that reads and prints   | Composed   | The two reports answer different questions and neither changes anything, so the wrapper runs its own checks and then the child's, passing that output through unmodified ([ADR-0045](../decisions/ADR-0045-compose-doctor-with-the-child-report.md)).                            |

Those are the only two resolutions available. Shadowing a child verb and leaving `--` as the sole remedy is not one of them, because a user reaching for a diagnostic does not know a wrapper is in the way. Every overlap is named in this table with the surface it shares and its reasoning, and the audit that keeps the table honest is the same one that covers flags ([ADR-0044](../decisions/ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md)).

A verb the wrapper has not yet built is not on the table above and not claimed. Until the slice that specifies its behaviour lands it, the spelling reaches the child like any other unclaimed token, which is what keeps the empty-claim rule stated for flags true of verbs as well.

There is no `init` and no wrapper-authored scaffold. Account setup is `account login`; profile and configuration setup starts from the [shipped examples](./configuration.md#generated-examples-and-schema), and flags or configuration select both launch axes. The wrapper never writes user configuration. See [ADR-0015](../decisions/ADR-0015-retire-the-init-verb.md) and [ADR-0091](../decisions/ADR-0091-refuse-an-unconfigured-first-launch-without-scaffolding.md).

## Passthrough contract

Forwarding is verbatim. Specifically:

| Property        | Rule                                                                                                                                            |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Order           | Preserved exactly. No reordering, no sorting, no moving flags before positionals.                                                               |
| Bytes           | Preserved exactly. Arguments are carried as OS strings and never round-tripped through UTF-8. A non-UTF-8 argument reaches the child unchanged. |
| Empty arguments | Preserved. An empty string is a real argument and is never filtered.                                                                            |
| Count           | Preserved. No deduplication, no splitting on whitespace, no joining.                                                                            |
| Quoting         | Untouched. The shell already removed quotes; the wrapper does not re-add or re-interpret them.                                                  |
| Unknown flags   | Forwarded. An unrecognized flag is a child flag by definition.                                                                                  |

There is no argv normalization step. A change that adds one is a change to the passthrough contract and requires a decision record before it requires code.

Standard input, standard output, and standard error are inherited by the child unmodified, under [the stream contract](./logging-and-output.md#the-stream-contract).

For a resolved profile, [ADR-0028](../decisions/ADR-0028-pass-composed-settings-with-the-native-flag.md) narrowly authorizes one wrapper-owned prefix, `--settings <absolute composed-settings path>`. Every user-supplied token remains an untouched suffix with order, bytes, count, and `--` sentinel preserved. The wrapper does not parse or normalize that suffix. What the child sees in `argv[0]`, and how the prefix itself is typed, are in [process runtime](./process-runtime.md#child-argument-vector).

## Parser shape

A derive-based parser cannot be the only gate, and the reason is specific.

Configuring a parser to accept unknown external subcommands makes it treat an unexpected positional token as a subcommand name. A leading unknown flag is not a positional: `claude-session-rs --print hello` is rejected as an unexpected argument before external-subcommand handling applies. Relaxed hyphen handling does not rescue this, because it applies to a declared value rather than to the top-level parse. Since a leading child flag is one of the most common passthrough invocations, the parser must not see it.

The contract is therefore:

1. Pre-split argv. A pure function scans the front of the argument vector, consuming only tokens the wrapper's denylist claims and their values. It stops at the first token that is not a wrapper flag, or at `--`.
2. Classify. If the next token is a wrapper verb, the remainder goes to the parser. Otherwise the remainder — including `--` handling — is child argv and is never handed to the parser.
3. Parse only the wrapper's part. The parser sees a grammar in which every token is one it defines.

The pre-split is pure and total over a list of OS strings, which is what makes it directly unit-testable. It is the single point where the passthrough contract can silently break, and the golden-argv tests in [testing and quality](./testing-and-quality.md) exist to guard it.

The split itself never fails. It classifies; only the wrapper's own parse and validation step exits. That keeps totality a property of the function rather than a claim about its callers, and it gives exactly three outcomes for a token that looks like a wrapper flag:

| Token                                                                                   | Outcome                                                                           |
| --------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Not an exact claimed spelling — `--configg`, `--acc`, `--CONFIG`, `-vq`                 | Split stops. The token and everything after it is child argv, forwarded verbatim. |
| Claimed, value missing — `--config` last, or before `--`, or before a `-`-leading token | `Usage`, naming the flag on standard error.                                       |
| Claimed, value malformed — `--config=`, `--verbose=1`                                   | `Usage`, naming the flag on standard error.                                       |
| Claimed, value not valid UTF-8 where the flag requires text — `--account=<bad bytes>`   | `Usage`, naming the flag. `--config` is exempt: its value is a path.              |

The wrapper never suggests that an unrecognized token was a mistyped wrapper flag. Suggestion machinery needs a model of the child's flags in order to know what it is not looking at, and the wrapper does not have one. Codes are in [exit codes](./exit-codes.md#wrapper-matrix).

Two parser settings are hazards rather than tools here. Long-argument inference is opt-in and stays off, since a unique prefix today is not a unique prefix after the child ships a flag. A trailing variable-argument declaration is not a substitute for the pre-split either, and it carries a live argv-corruption risk: a value delimiter configured on that argument still applies, which would split a child token on a character the user typed literally.

The parser is additionally configured to disable its automatic version flag, so that `--version` reports both the wrapper and the resolved child rather than the wrapper alone.

## Help

`--help` output is generated by the parser. A hand-maintained flag table is forbidden: it drifts from the parser the first time a flag is added and the table is not.

Authored prose the parser cannot generate — worked passthrough examples, the `--` explanation, the environment variable that turns colour off, a pointer to this documentation — lives in a text file under the output module and is included into the parser's long help at compile time.

Colour is the one behaviour a user can change that has no flag to discover it by, because the spelling belongs to the child ([presentation](./presentation.md#colour)). Help is therefore where it is named, and the naming is one line: a convention the wrapper honours, not a wrapper setting.

The generated part describes the wrapper's grammar only. It does not reproduce, summarize, or link into the child's flag list, because that list is not the wrapper's to track. It should say, once and plainly, that unrecognized arguments are forwarded.

The child's list is not reproduced because it is delegated. `--help` is a claimed spelling over a read-only surface, so it composes: the wrapper's generated help, then `claude --help` under [the composed-output delimiter](./logging-and-output.md#composed-output). This is what makes the wrapper's answer a superset of the child's rather than a replacement for it, which is the ground [ADR-0044](../decisions/ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md) keeps the collision on. The child's help is never parsed, so nothing here tracks its grammar.

The `help` verb is the same surface under another spelling: `claude-session-rs help [<verb>]` prints exactly what `--help` and `<verb> --help` print. A verb's help composes on the same test: `help doctor` appends `claude doctor --help`, because `doctor` is the one verb whose name the child also owns. `account` appends nothing — it was renamed precisely so there is no shared surface — and neither does any verb the child does not have. Requested help is a result — standard output, exit `0`. Help printed because an invocation was malformed is a diagnostic — standard error, exit `Usage`. The parser's own default differs on both counts and is overridden; see [exit codes](./exit-codes.md#wrapper-matrix).

A namespace verb requires its subcommand. `account`, the only one, satisfies no invocation on its own, so bare `account` is malformed: the verb's help is a diagnostic, and so is an unrecognized subcommand. Both exit `Usage`. Unlike a mistyped wrapper flag, an unrecognized subcommand carries a nearest-match suggestion — the parser's subcommand set is closed and wholly wrapper-owned, so the reasoning that denies one to [flag spelling](#flag-spelling) does not reach it. See [ADR-0052](../decisions/ADR-0052-require-an-explicit-subcommand.md).

Shell completions cover the wrapper's grammar for the same reason. Completions never attempt to complete child arguments.

```text
claude-session-rs completion <bash|elvish|fish|powershell|zsh>
```

The five are the full set the generator supports, so the list is the dependency's rather than a subset this project would have to justify and revisit. The script is the verb's result and is written raw to standard output: no header, no summary, no diagnostic. An unrecognized shell exits `Usage`.

Man pages are generated from the same parser tree. Because help, completions, and man pages all read one `Command` tree, the flag list has a single source and no surface can drift from another. The authored prose file included into long help is included into the man page too. See [ADR-0016](../decisions/ADR-0016-ship-man-pages.md).

```text
claude-session-rs man
```

`man` writes one roff page — the wrapper's own — to standard output, so a user previews with `man -l -` and a packager renders `claude-session man > claude-session.1` at build time. Per-verb pages need a named output directory, which is deferred until a packager needs one ([ADR-0086](../decisions/ADR-0086-emit-one-man-page-to-standard-output.md)); each verb's detail stays reachable through its own requested help. The verb takes no `--json`: roff is not data.

The page states an `about` and a version that `--help` never shows, because the authored long help shadows the one and the wrapper's own `--version` composes the other. Left to the parser's defaults the `NAME` section — what `whatis` and `apropos` index — would carry an internal phrase, so both are set explicitly on the root command.

## Version output

`--version` and the `version` verb compose wrapper and child output on standard output:

```text
  This is claude-session-rs 0.1.0.
  It wraps the claude at /usr/local/bin/claude, whose own version follows.


--- claude --version ---

1.0.2
```

The wrapper's own sentences are followed by the shared composed-output delimiter and the child's native `--version` bytes unchanged. Reporting both is the point: a user debugging wrapper behaviour needs to know the wrapper and child versions, while the delimiter makes their ownership explicit.

When the child cannot be resolved or its version cannot be read, one condition line replaces the child section and the exit stays `0`. The wrapper's version is a fact it always knows; refusing to report it because the child is missing would withhold the one answer the user came for. `doctor` is where a missing child fails.

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

Without a terminal, a confirming verb fails rather than prompting or proceeding. When no controlling terminal is available and no escape was given, the verb stops before any side effect and exits `Unavailable` (69). The diagnostic names the escape above; token ingestion through `--stdin` follows [ADR-0027](../decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md).

Reading the absence of a terminal as consent is the alternative, and it makes `account remove` silent under a pipe. Prompting anyway is worse: the process hangs on a stream nobody is reading. See [ADR-0021](../decisions/ADR-0021-fail-closed-without-a-terminal.md).

### The predicate

A terminal is available when the process can open `/dev/tty` read-write. The `open` is the test; nothing consults `isatty(0)`. So `something | claude-session account remove work` prompts — standard input is a pipe, but the controlling terminal is still there and no confirming verb reads standard input. `Unavailable` is for the case where the `open` itself fails: a cron job, a service unit, a container with no terminal, a session detached by `setsid`. See [ADR-0053](../decisions/ADR-0053-read-a-confirmation-from-the-controlling-terminal.md).

The prompt and its answer use that same handle, which is why a confirmation is [the one exception](./logging-and-output.md#the-stream-contract) to prompts going to standard error.

### The exchange

```text
This deletes everything this wrapper stored for "work", and revokes nothing at the provider.
Remove it? [y/N]
```

`y` and `yes` consent, case-insensitively and after trimming surrounding whitespace. Everything else declines — a bare Enter, an unrecognized answer, and end of input alike. There is one question and one answer; an unrecognized answer is not re-asked, because a verb that loops on a terminal it may not fully control is a verb that can hang.

The capitalized letter is the default, and it is `N` because the verb is destructive. The prompt names the object and the consequence, so what a person approves and what `--json` reports are the same facts.

Declining is not a failure. The verb stops before any side effect and exits `0`: nothing was removed, which is an outcome rather than an error. What the report says is owned by the verb — for `account remove`, [accounts](./accounts.md#removal).

### `--yes` and `--json` are orthogonal

`--json` selects a schema, never a mode. It never supplies consent and never suppresses the prompt:

| Invocation                      | Terminal available                                  | No controlling terminal                 |
| ------------------------------- | --------------------------------------------------- | --------------------------------------- |
| `account remove work`           | Prompt; either answer exits `0` with its report     | `Unavailable`, before any side effect   |
| `account remove work --json`    | Prompt on the terminal, then one document on stdout | `Unavailable`, error document on stderr |
| `account remove work --yes`     | No prompt                                           | No prompt                               |
| `--yes --json`, in either order | No prompt; one document                             | No prompt; one document                 |

Because the prompt is not on standard output, `--json` needs no interaction rule: `account remove work --json | jq` is already clean. Making `--json` imply `--yes` would let a formatting flag destroy data, and making it refuse rather than prompt would buy a third interactivity mode that discriminates nothing the predicate above has not already decided ([ADR-0051](../decisions/ADR-0051-let-every-surface-element-discriminate.md)). This is [ADR-0017](../decisions/ADR-0017-declare-a-human-facing-cli.md) and [ADR-0024](../decisions/ADR-0024-machine-output-is-a-per-verb-flag.md) applied, not a new rule.

### Why `--yes` is not in the flag table

`--yes` is a verb-level flag, accepted after the verb:

```text
claude-session-rs account remove work --yes
```

Its absence from the wrapper-owned flag table above is the design, not an oversight. A top-level flag is intercepted before the passthrough split and is therefore subtracted from the child's reachable surface for good; a flag appearing after a wrapper verb is parsed inside an invocation the child never sees, so it costs the child nothing. `--json` and `doctor --list` are verb-level for the same reason.

There is no `--non-interactive`. Detecting the missing terminal already produces exactly that behaviour, so a flag requesting it would be surface bought for nothing.

Passthrough is untouched. The wrapper does not inspect standard input on a passthrough invocation, and the child's own prompting is the child's business.

## Exit behaviour

Wrapper verbs exit with codes from the wrapper's matrix. A passthrough invocation exits with the child's status. The two regimes and the boundary between them are in [exit codes](./exit-codes.md).

## Further reading

- [clap `Command` documentation](https://docs.rs/clap/latest/clap/struct.Command.html)
- [Command Line Interface Guidelines](https://clig.dev/)
