# Configuration

Two distinct things share the word "configuration" in this project, and keeping them apart is the first thing to understand.

**The wrapper's own configuration** controls `claude-session`: which child to run, which account to use, how verbose to be. It is a layered value resolved at startup.

**The child's settings** are what `claude` reads from its account-wide configuration directory and an additional per-group document. `claude-session` generates the latter by composing user-authored pieces.

**The child's authentication precedence** is separate from wrapper configuration precedence. It determines whether ambient cloud, API, helper, injected subscription-token, or saved-login authentication wins. [Accounts](./accounts.md#stored-modes-and-launch-behavior) owns that operational contract.

Paths for both are in [XDG storage](./xdg-storage.md).

This describes normative design. The crate is pre-implementation.

## The wrapper's configuration

### Precedence

Later layers override earlier ones:

1. **Built-in defaults** — compiled in. Every key has one, so a missing configuration file is never an error.
2. **User configuration file** — under the config base directory.
3. **Project configuration file** — discovered by walking up from the working directory, for per-repository overrides.
4. **Environment variables** — see below.
5. **Command-line flags** — highest. The user typed it just now.

A missing file at any layer is **not an error**. An unreadable or malformed file _is_ an error, reported with the path — the distinction is between "you did not configure this" and "you tried to and it did not work".

`--config <path>` replaces the user layer with an explicit file. Because it must be honoured before configuration exists, it is read from the raw argument vector rather than from the resolved value.

### Environment variables

| Property     | Rule                                                                                  |
| ------------ | ------------------------------------------------------------------------------------- |
| Prefix       | `CLAUDE_SESSION_`                                                                     |
| Nesting      | Double underscore separates levels: `CLAUDE_SESSION_OUTER__INNER` sets `outer.inner`. |
| Case         | Upper-case in the environment, lower-case snake in the file                           |
| Empty values | Treated as set-to-empty, not as unset. Unsetting means removing the variable.         |

A **single** underscore is part of a key name, not a level separator. `CLAUDE_SESSION_CHILD_BIN` therefore sets the flat key `child_bin` — the child-binary override in [process runtime](./process-runtime.md) — and not a nested `child.bin`.

Internal variables — the recursion marker, and any other `CLAUDE_SESSION_*` key the wrapper sets for its own purposes — are **not** configuration keys, and are scrubbed from the child's environment. See [process runtime](./process-runtime.md).

### Future token-helper boundary

A future `token_helper` setting may select an argv-based helper process. Its settled boundary is:

- argv execution with no shell interpolation;
- explicit selection;
- no silent fallback from helper to file or file to helper.

The command protocol and configuration schema remain deferred under [ADR-0029](../decisions/ADR-0029-use-a-credential-helper-process-boundary.md). No generated example field exists until that specification is accepted.

### Schema

- Unknown keys are **rejected**, not ignored. A typo in a configuration file is the single most common configuration bug, and silently ignoring it produces a program that does not do what its configuration says.
- The rejection names the offending key, its file, and, where the distance is small, the key it was probably meant to be.
- The resolved value is **immutable**. It is built once and passed by shared reference. Nothing mutates configuration mid-run.
- Every key has a documented default, a type, and a one-line meaning. That description lives on the field itself, in the type, and is the source the artifacts below are rendered from — never a parallel doc that can rot.

`claude-session config` prints the resolved value, including which files were consulted and which existed; see [Commands](#commands).

### Generated examples and schema

The wrapper never writes the user's configuration ([ADR-0006](../decisions/ADR-0006-place-files-by-xdg-ownership.md)), so it cannot scaffold a starter file. It ships one to **copy** instead, generated from the config types so it cannot drift ([ADR-0013](../decisions/ADR-0013-generate-config-examples-from-types.md)).

Four artifacts live under `examples/`, and which are generated follows from whether a type describes them:

| Artifact                         | Rendered from                                       | Kind                |
| -------------------------------- | --------------------------------------------------- | ------------------- |
| `examples/config.example.toml`   | the wrapper's configuration type                    | Generated           |
| `examples/config.schema.json`    | the wrapper's configuration type                    | Generated           |
| `examples/manifest.example.yaml` | the manifest type — `layers` and the strategy table | Generated           |
| `examples/piece.example.json`    | nothing — a piece is the child's own format         | **Hand-maintained** |

A piece has no type to reflect over, because its shape is the child's and evolves on the child's schedule. It therefore ships as an authored file under the _same_ discipline as the generated ones: a header, fake values, and copied rather than scaffolded. The only difference is what keeps it correct.

#### What a generated example contains

- **Required keys active, optional keys commented out.** The uncommented file is a minimal valid configuration; uncommenting adds optional surface.
- **Every key annotated with its own description**, taken from the field in the type.
- **Placeholders that are obviously fake** — `REPLACE_ME`, `/path/to/thing`, the first enum variant. A placeholder that happens to be a valid live value invites accidental use.
- **A header** naming the copy destination and stating that the wrapper never writes configuration.

Generation **fails** when a public field carries no description. That hard failure is the whole mechanism: it is what keeps the example self-documenting instead of a wall of bare keys, and it means adding a field without documenting it cannot pass review.

The generated example **must round-trip through the real loader** in a test. An example the program itself would reject is worse than none; see [testing and quality](./testing-and-quality.md).

#### Freshness

Generated files rot silently unless something proves they still match the types. The generator renders every artifact in memory and **compares it byte for byte with what is on disk**; a file whose contents differ is stale, and a missing file is stale.

This is deliberately **not a cache**. There is no hash file, no timestamp, and nothing to invalidate — rendering is deterministic, so identical types produce identical bytes, and a commit that changes no field is a natural no-op. A cache keyed on the model would add an artifact to commit, an invalidation rule to get wrong, and a failure mode where the cache says fresh and the file is not.

Two modes, one command:

| Mode                             | Behaviour                                                   |
| -------------------------------- | ----------------------------------------------------------- |
| `cargo xtask gen-config`         | Rewrites stale artifacts and stages exactly those paths     |
| `cargo xtask gen-config --check` | Reports stale artifacts and exits non-zero, writing nothing |

The default mode runs as a pre-commit hook, so a type change and its regenerated example land in the same commit; `--check` runs in CI. The hook passes no filenames and always runs, because an example's relationship is to the whole model rather than to any one changed file.

One consequence is worth knowing before it bites: **generated files must be staged whole.** Partially staging one — `git commit -p` on a generated example — commits something the generator did not produce, and the gate cannot tell that apart from a stale file.

The generator lives in an `xtask` workspace member rather than in the shipped binary, so schema machinery never reaches a user's install ([ADR-0014](../decisions/ADR-0014-xtask-workspace-for-dev-tooling.md)).

### Provenance

For each key, the wrapper tracks which layer supplied the winning value. This is what makes "why is it doing that?" answerable in one command rather than by bisecting files. `config` reports it.

## Composing the child's settings

The child may own `config/settings.json` in the account-wide configuration directory as its base layer. The user authors wrapper **pieces** and a **manifest**; the wrapper composes them into `groups/<group>/settings.json`, supplied as an additional native `--settings` layer under [ADR-0028](../decisions/ADR-0028-pass-composed-settings-with-the-native-flag.md).

### Inputs

**Pieces** are partial settings documents in the child's own format, JSON, under `settings/` in the config base. Each is a fragment: a piece that only sets one key contains only that key. Pieces are read-only to the wrapper.

**Manifests** are profiles. One YAML file per profile under `manifests/`, whose sole required field is an ordered, non-empty list of piece names:

```yaml
# manifests/work.yaml
layers:
  - base
  - work-permissions
  - verbose-logging
```

Order is significant and later wins. Unknown fields in a manifest are rejected. An empty list is rejected. A referenced piece that does not exist is an error naming both the manifest and the resolved path it looked for.

The active profile comes from `--profile`, then the environment, then the wrapper's configuration, then a default.

### Merge semantics

Pieces are folded left to right into one document:

| Node type             | Rule                                                                      |
| --------------------- | ------------------------------------------------------------------------- |
| Object                | Merged key by key, recursively                                            |
| Scalar                | Last writer wins                                                          |
| Array                 | **Replace** by default                                                    |
| Array, `concat`       | Elements appended in layer order                                          |
| Array, `merge-by-key` | Elements matched on a named field and merged; unmatched elements appended |

Array strategy is the one genuinely contested decision. Replace is the default because it is predictable: what the last piece says is what you get. Concatenation is what you want for additive lists — extra permitted paths, extra tools — and it is opt-in **per key** through a strategy table in the manifest, because a global concat setting is wrong for roughly half of any real settings file.

A **type conflict** — one piece making a key an object and another a string — is an error, not a silent overwrite. It names the key path, both pieces, and both types.

Merging is **deterministic**: the same inputs produce byte-identical output, with object keys in a stable order. A generated file that reshuffles on every run defeats the freshness check and makes diffs useless.

### Provenance sidecar

Alongside the generated settings, the wrapper writes a sidecar recording the manifest used, the ordered pieces with their resolved paths, and, for every leaf key, which piece set it.

This is the difference between "the setting is wrong" and "the setting is wrong _because_ this piece overrode that one". `config` surfaces it.

### Generation and freshness

Generation resolves the profile, loads the pieces, merges, validates, and writes atomically to `accounts/<account>/groups/<group>/settings.json`.

The freshness check compares the modification time of the generated file against **the manifest and every referenced piece**. Checking only the manifest is a real bug: editing a piece without touching the manifest leaves stale settings in place, and the symptom — an edit that appears to do nothing — is genuinely hard to diagnose.

### Validation

Validation is deliberately **pragmatic**. The wrapper validates the structure it owns and the well-formedness of the whole. It does not reject unknown keys in the child's settings, because the child's schema evolves on its own schedule and a wrapper that rejects a valid new setting is worse than one that passes it through.

That is the opposite of the rule for the wrapper's own configuration, and the asymmetry is the point: strict about what we own, permissive about what we forward. It is the passthrough contract applied to configuration.

Unknown _piece_ keys are surfaced as warnings with provenance rather than errors.

### Child-owned account state

Trust, onboarding, project history, and other native state remain child-owned in the shared account `config/`. The wrapper neither seeds nor synchronizes them.

## Commands

Two verbs, no subcommands ([ADR-0049](../decisions/ADR-0049-collapse-config-inspection-into-one-verb.md)).

| Command        | Reports                                                                                                                                                                                                     |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `config`       | The resolved wrapper configuration with per-key provenance; which files were consulted and which existed; the active profile, its resolved pieces, and generated-settings freshness; and every defect found |
| `profile list` | Available manifests                                                                                                                                                                                         |

Both accept `--json`, and both write data to standard output and diagnostics to standard error; see [logging and output](./logging-and-output.md). `config` reports one profile's ordered layers and resolved paths when given `--profile <name>`.

`config` **validates**, so it is an assertion verb: a structural defect or type conflict exits with that defect's code, while unknown-piece-key warnings stay advisory at `0`. The exact table is in [exit codes](./exit-codes.md#inspection-verbs-and-assertion-verbs).

The checks it runs are the config-scoped subset of the one probe catalog `doctor` runs in full, so the two cannot disagree and quote one remediation wording ([ADR-0018](../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)). `doctor` reports health and never renders configuration.

## Further reading

- [`figment`](https://docs.rs/figment/)
- [`serde_json`](https://docs.rs/serde_json/)
- [RFC 7396: JSON Merge Patch](https://www.rfc-editor.org/rfc/rfc7396.html)
