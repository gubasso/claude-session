# Presentation

How every human-facing byte looks. Which stream carries it is [logging and output](./logging-and-output.md)'s to say; this page owns appearance, and every verb's renderer satisfies what follows.

Every surface the wrapper owns is rendered under it: the reports of `doctor`, `account`, `profile`, and `config`, the error diagnostic, the standard-error log mirror, and the composed-output delimiter. The contract is written for every renderer because a rule discovered while building one verb binds the next one, whose author has no reason to read the first verb's page ([ADR-0081](../decisions/ADR-0081-bind-every-human-surface-to-one-presentation-contract.md)).

## The contract

1. Plain text is complete. Colour and weight are decoration: strip them and nothing a reader needs is gone. A redirected stream, a colour-blind reader, and a screen reader all lose them, and none of the three may lose information.
2. Status is a word. `[pass]`, `[warn]`, `[fail]`, `[skipped]` — never an emoji, a glyph, or a colour standing in for one.
3. Layout does not depend on the terminal. No padding to the width, no reflow, no cursor movement. A command produces the same bytes into a pipe and into a terminal, which is the property a golden test can pin and a script can rely on.
4. One renderer writes it. Every human byte goes through the output writer named in [the stream contract](./logging-and-output.md#the-stream-contract), so colour, `--quiet`, and JSON mode behave identically across verbs instead of being re-decided per command.
5. Machine output carries no decoration. No `--json` document and no log record contains an escape byte.
6. The wrapper decorates only what it wrote. The child's bytes pass through unchanged, in appearance as in content.
7. The human format is written for a person ([ADR-0093](../decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md)). A reader learns what happened, what it costs them, and what to do next, in sentences. Internal identifiers, `key=value` fields, counters, and exit codes belong to the machine format, which already carries every one of them — so removing them from human output loses a caller nothing, and `--json` is where a caller was always meant to look.

Rule 7 is a floor, not a licence: a next action is named only where one exists, and a check that has no honest remedy still says so plainly rather than inventing one. Where rule 7 and rule 1 could disagree, rule 1 wins — an explanation is text, never a colour.

### Wrapping and columns

A renderer that pads or wraps does so at a constant, so rule 3 holds: status words pad to a fixed column, and prose wraps at column 76. Both are the same in a pipe and in a terminal, which is the property the golden tests pin. Reading `COLUMNS` or the terminal size to do either is the thing rule 3 forbids.

Both constants live in one module every renderer imports, so a second renderer cannot pick a second column. Text the wrapper did not compose — a parser's own usage block, which arrives already laid out — passes through with its lines intact rather than reflowed, because reflowing a grammar destroys it.

## Colour

The named human surfaces are colourful by default when their actual destination is a capable terminal. JSON and logs are always undecorated; redirected and dumb-terminal destinations fail closed. Machine mode dominates every environment override.

Applied to standard error text and to standard output only in human format. Never in JSON mode. Resolved in this order, first match wins:

1. `FORCE_COLOR` is present and not an empty string — on, whatever the stream is. Per [the convention](https://force-color.org/), the value is not read, so `FORCE_COLOR=0` forces colour on rather than off. The spelling is a request to force, not a boolean.
2. `NO_COLOR` is present and not an empty string — off. Per [the convention](https://no-color.org/), which fixes both halves of that test: an unset variable and an empty one are alike inert, and any other value disables colour whatever it says.
3. `TERM` is `dumb` — off. Neither convention site names this rung; [Command Line Interface Guidelines](https://clig.dev/#colour) does, and a terminal declaring itself dumb cannot render the escapes.
4. The target stream is not a terminal — off.
5. Otherwise — on.

Presence and emptiness are the whole test on rungs 1 and 2. Reading the value would invent a fourth convention for a question two published ones already answer: under the published `NO_COLOR` convention, `NO_COLOR=0` disables colour. When both variables are active, rung 1 wins and colour is on.

### What the ladder does not read

A reader holding the wider convention will look for these, so their absence is stated rather than left to be inferred ([ADR-0083](../decisions/ADR-0083-read-only-the-two-published-colour-variables.md)):

| Input                  | Why the ladder does not read it                                                      |
| ---------------------- | ------------------------------------------------------------------------------------ |
| `CLICOLOR_FORCE`       | A second spelling of the force override, which rung 1 already honours                |
| `CLICOLOR=0`           | A weaker no-colour override, which rung 2 already honours                            |
| `CLICOLOR=1`           | The terminal test, which rung 4 already is                                           |
| `CI`, `GITHUB_ACTIONS` | Sniffing them repeats rung 4, and strips colour from the runners that do render ANSI |
| A `--color` flag       | The spelling belongs to the child; see below                                         |

A general command-line checklist would require the `--color auto|always|never` flag, and this project deliberately does not ship it. There is no wrapper flag for colour. `NO_COLOR` is the established convention and costs the child nothing, whereas claiming `--no-color` would take that spelling away from the child for good — a passthrough-contract change needing its own decision record, per [the CLI surface](./cli-surface.md) and [ADR-0003](../decisions/ADR-0003-reserve-a-small-wrapper-cli-surface.md).

### Coloured surfaces

The set is closed ([ADR-0082](../decisions/ADR-0082-colour-a-closed-set-of-named-surfaces.md)). A surface absent from it is written plain, and adding one is an edit to a page rather than a renderer's judgement. Rule 1 above is why every row carries nothing:

| Surface                                 | Stream                    | Carries                                         |
| --------------------------------------- | ------------------------- | ----------------------------------------------- |
| The `error[Kind]` token in a diagnostic | stderr                    | Nothing the kind does not already spell         |
| The level word in the stderr log mirror | stderr                    | Nothing the level does not already spell        |
| The composed-output delimiter line      | stdout, human format only | Nothing the command name does not already state |
| A status word in any report row         | stdout, human format only | Nothing the bracketed word does not already say |
| A section heading in any report         | stdout, human format only | Nothing the heading text does not already say   |

The last two rows are written per shape rather than per verb, because the shape is what carries the rule: green, yellow, and red follow `pass`, `warn`, and `fail`, a skip is dim, and a heading is bold. A renderer that emits neither shape emits no colour. Nothing else in a report is coloured, because nothing else would be carrying its own meaning.

Colour is emitted as four-bit SGR by the one renderer. Eight-bit and true-colour sequences buy a shade a terminal may not have, for a decoration rule 1 already says carries nothing.

## Tables, progress, and prompts

One table is rendered, by [`session list`](./sessions.md#commands), and the page that owns the verb owns the surface. Every column pads to the widest cell in that report's own rows, the column name included, and the last column is not padded; nothing reads the terminal and nothing is cut to fit one, so rule 3 holds and the rows into a pipe are the rows into a terminal. Column names and the rule under them carry no colour: a column name spells its column, a rule spells nothing, and the status token and the heading stay a report's only decorated parts. A second table comes through the verb that needs it and is laid out by the same shared function, because two layout functions would put two tables at two shapes.

The wrapper renders no progress indicator. One is forbidden during a passthrough for the reason a banner is: standard output belongs to the child, and standard error already carries the child's own diagnostics. Rule 3 rules out a spinner everywhere else, since it works by moving the cursor.

The one prompt each of `account remove`, `account login --token`, and `session clean` raises reaches [the controlling terminal](./cli-surface.md#the-predicate) rather than either standard stream, and states what answering costs before it asks. An interactive prompt of any other shape arrives through the verb that needs it, which names the surface on its own page and admits any crate through [the dependency procedure](./dependencies.md#adding-a-dependency).

## Where the rule stops

Three surfaces a reader will meet are outside this contract, and their absence is stated rather than left to be inferred:

- The child's own bytes, which pass through unchanged under rule 6.
- The generated completion scripts and man page, whose shapes belong to the shells and to `roff`. The prose inside them — every `about` string and the wrapper's own help text — is authored for a person and is governed; the layout around it is not. The same holds for the usage block `clap` renders into a `Usage` diagnostic, which passes through with its lines intact.
- The developer task runner under `xtask`, which no user runs and which writes to its own streams.

## Further reading

- [`no-color.org`](https://no-color.org/) and [`force-color.org`](https://force-color.org/)
- [Command Line Interface Guidelines](https://clig.dev/)
