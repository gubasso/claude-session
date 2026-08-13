# Configuration

Two distinct things share the word "configuration" in this project, and keeping them apart is the first thing to understand.

The wrapper's own configuration controls `claude-session`: which child to run, which account to select, which profile to compose. It is a layered value resolved at startup. Verbosity is not among them — it is invocation-scoped, through [`--verbose` and `--quiet`](./cli-surface.md#wrapper-owned-flags).

The child's settings are what `claude` reads from its account-wide configuration directory and an additional per-profile document. `claude-session` generates the latter by composing user-authored pieces.

The child's authentication precedence is separate from wrapper configuration precedence. It determines whether ambient cloud, API, helper, injected subscription-token, or saved-login authentication wins. [Accounts](./accounts.md#stored-modes-and-launch-behavior) owns that operational contract.

Paths for both are in [XDG storage](./xdg-storage.md).

The three-key wrapper configuration loader and provenance model are implemented, including last-used account fallback with `flag`, `environment`, `user-config`, `marker`, or `none` provenance. Profile resolution, piece resolution, ordered multi-piece composition, per-key array strategies, the full provenance sidecar including its `keys` contributor map, the `profile` and `config` verbs, and the generated examples and schema are also implemented. This page describes no unimplemented behaviour.

## The wrapper's configuration

### Precedence

Later layers override earlier ones:

1. Built-in defaults — compiled in. Every key has one, so a missing configuration file is never an error. Every key is optional, so every default is unset.
2. User configuration file — under the config base directory.
3. Project configuration file — `.claude-session-rs.toml`, discovered by [walking up from the working directory](#project-file-discovery), for per-repository overrides. It may set [`default_profile`](#keys) and nothing else.
4. Environment variables — see below.
5. Command-line flags — highest. The user typed it just now.

A missing file at any layer is not an error. An unreadable or malformed file is an error, reported with the path — the distinction is between "you did not configure this" and "you tried to and it did not work".

`--config <path>` replaces the user layer with an explicit file. Because it must be honoured before configuration exists, it is read from the raw argument vector rather than from the resolved value. It names one file, not a mode: project discovery still runs.

#### Project file discovery

The search starts at the working directory and walks upward. The first `.claude-session-rs.toml` found wins — files are not unified across directories. The walk stops at the enclosing repository root, the directory holding a `.git` entry, which is a file for worktrees and submodules and a directory otherwise. Git itself is never invoked. Outside a repository there is no project layer at all.

The stop rule follows the layer's purpose: these are per-repository overrides, so the repository is the boundary. It needs no marker key, no ceiling variable, and no merge-many rule ([ADR-0070](../decisions/ADR-0070-discover-the-project-configuration-file-at-the-repository-root.md)). Nested repositories stop at the inner one.

### Environment variables

| Property     | Rule                                                                          |
| ------------ | ----------------------------------------------------------------------------- |
| Prefix       | `CLAUDE_SESSION_RS_`                                                          |
| Case         | Upper-case in the environment, lower-case snake in the file                   |
| Empty values | Treated as set-to-empty, not as unset. Unsetting means removing the variable. |

Every key is flat, so an underscore is always part of a key name and never a level separator: `CLAUDE_SESSION_RS_CHILD_BIN` sets `child_bin`, not a nested `child.bin`. The per-key spellings are in [the key table](#keys). A separator convention is specified when a nested key first exists, and not before ([ADR-0051](../decisions/ADR-0051-let-every-surface-element-discriminate.md)).

Internal variables — the recursion marker, and any other `CLAUDE_SESSION_RS_*` key the wrapper sets for its own purposes — are not configuration keys. What reaches the child is [process runtime](./process-runtime.md#child-environment)'s to say.

The wrapper also reads variables outside the prefix, and none of them is configuration: `NO_COLOR`, `FORCE_COLOR`, and `TERM` are read by [the colour ladder](./presentation.md#colour), `RUST_LOG` by [verbosity](./logging-and-output.md#verbosity), and the XDG bases by [XDG storage](./xdg-storage.md). They are listed here because this is where a reader asks what the environment does, and owned there because that is where they act. None has a file spelling, a layer, or a row above — a key is a permanent contract this project defines, and these are conventions it honours.

### Future token-helper boundary

A future `token_helper` setting may select an argv-based helper process. Its settled boundary is:

- argv execution with no shell interpolation;
- explicit selection;
- no silent fallback from helper to file or file to helper.

The command protocol and configuration schema remain deferred under [ADR-0029](../decisions/ADR-0029-use-a-credential-helper-process-boundary.md). No generated example field exists until that specification is accepted.

### Keys

Three keys. All optional; the default of each is unset.

| Key               | Type          | Unset means                          | Environment                         | Layers                     | Meaning                                                                                                |
| ----------------- | ------------- | ------------------------------------ | ----------------------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------ |
| `child_bin`       | absolute path | search `PATH`                        | `CLAUDE_SESSION_RS_CHILD_BIN`       | user, environment          | The child to run ([process runtime](./process-runtime.md#child-resolution))                            |
| `default_account` | identifier    | fall through to the last-used marker | `CLAUDE_SESSION_RS_DEFAULT_ACCOUNT` | user, environment          | The account when `--account` is absent ([accounts](./accounts.md#selection))                           |
| `default_profile` | identifier    | report no profile; refuse a launch   | `CLAUDE_SESSION_RS_DEFAULT_PROFILE` | user, project, environment | The profile when `--profile` is absent ([selecting the active profile](#selecting-the-active-profile)) |

Identifiers follow [the identifier rules](./xdg-storage.md#identifiers); an absolute path is validated where it is used.

The project layer may set `default_profile` only. A repository that could set `child_bin` would choose the executable that runs, and one that could set `default_account` would choose the credential it runs under — both before the user has read a line of it. Either key in a project file is `Config`, not a silent ignore ([ADR-0071](../decisions/ADR-0071-restrict-the-project-layer-to-the-profile-key.md)).

No other key earns a row. Verbosity is invocation-scoped, colour is `NO_COLOR` ([presentation](./presentation.md#colour)), and `token_helper` stays deferred by [ADR-0029](../decisions/ADR-0029-use-a-credential-helper-process-boundary.md). A key is a permanent contract, so it is added by a present need rather than by symmetry ([ADR-0051](../decisions/ADR-0051-let-every-surface-element-discriminate.md)).

### Schema

- Unknown keys are rejected, not ignored. A typo in a configuration file is the single most common configuration bug, and silently ignoring it produces a program that does not do what its configuration says.
- The rejection names the offending key, its file, and, where the distance is small, the key it was probably meant to be. It exits [`Config`](./exit-codes.md#wrapper-matrix), in the [four-part shape](./exit-codes.md#error-message-shape) every wrapper diagnostic takes.
- The resolved value is immutable. It is built once and passed by shared reference. Nothing mutates configuration mid-run.
- Every key has a documented default, a type, and a one-line meaning. That description lives on the field itself, in the type, and is the source the artifacts below are rendered from — never a parallel doc that can rot.

`claude-session-rs config` prints the resolved value, including which files were consulted and which existed; see [Commands](#commands).

### Generated examples and schema

The wrapper never writes the user's configuration ([ADR-0006](../decisions/ADR-0006-place-files-by-xdg-ownership.md)), so it cannot scaffold a starter file. It ships one to copy instead, generated from the config types so it cannot drift ([ADR-0013](../decisions/ADR-0013-generate-config-examples-from-types.md)).

Four artifacts belong under `docs/reference/examples/`, and which are generated follows from whether a type describes them:

| Artifact               | Rendered from                                      | Kind            | Present |
| ---------------------- | -------------------------------------------------- | --------------- | ------- |
| `config.example.toml`  | the wrapper's configuration type                   | Generated       | Yes     |
| `config.schema.json`   | the wrapper's configuration type                   | Generated       | Yes     |
| `profile.example.yaml` | the profile type — `layers` and `array_strategies` | Generated       | Yes     |
| `piece.example.json`   | nothing — a piece is the child's own format        | Hand-maintained | Yes     |

All four exist. The three generated ones are rendered by the [`xtask` member](../explanation/architecture.md#one-shipped-crate-plus-xtask) from the configuration key descriptor beside the type, and the freshness rule below compares them byte for byte on every commit.

A piece has no type to reflect over, because its shape is the child's and evolves on the child's schedule. It therefore ships as an authored file under the same discipline as the generated ones: a header, fake values, and copied rather than scaffolded. The only difference is what keeps it correct.

#### What a generated example contains

- Required keys active, optional keys commented out. The uncommented file is a minimal valid configuration; uncommenting adds optional surface.
- Every key annotated with its own description, taken from the field in the type.
- Placeholders that are obviously fake — `REPLACE_ME`, `/path/to/thing`, the first enum variant. A placeholder that happens to be a valid live value invites accidental use.
- A header naming the copy destination and stating that the wrapper never writes configuration.

Generation fails when a public field carries no description. That hard failure is the whole mechanism: it is what keeps the example self-documenting instead of a wall of bare keys, and it means adding a field without documenting it cannot pass review.

The generated example must round-trip through the real loader in a test. An example the program itself would reject is worse than none; see [testing and quality](./testing-and-quality.md).

#### Freshness

Generated files rot silently unless something proves they still match the types. The generator renders every artifact in memory and compares it byte for byte with what is on disk; a file whose contents differ is stale, and a missing file is stale. The comparison is the whole trigger — there is no list of what causes regeneration, because such a list is the invalidation rule the next paragraph rejects.

Nothing at run time depends on freshness. These four are repository documentation: the installed binary never opens them, no probe checks them, and no exit code can report them. A stale artifact is a build-gate failure and nothing else.

This is deliberately not a cache. There is no hash file, no timestamp, and nothing to invalidate — rendering is deterministic, so identical types produce identical bytes, and a commit that changes no field is a natural no-op. A cache keyed on the model would add an artifact to commit, an invalidation rule to get wrong, and a failure mode where the cache says fresh and the file is not.

Two modes, one command:

| Mode                             | Behaviour                                                   |
| -------------------------------- | ----------------------------------------------------------- |
| `cargo xtask gen-config`         | Rewrites stale artifacts and stages exactly those paths     |
| `cargo xtask gen-config --check` | Reports stale artifacts and exits non-zero, writing nothing |

The default mode runs as a pre-commit hook, so a type change and its regenerated example land in the same commit; `--check` runs in CI. The hook passes no filenames and always runs, because an example's relationship is to the whole model rather than to any one changed file.

One consequence is worth knowing before it bites: generated files must be staged whole. Partially staging one — `git commit -p` on a generated example — commits something the generator did not produce, and the gate cannot tell that apart from a stale file.

The generator lives in an `xtask` workspace member rather than in the shipped binary, so schema machinery never reaches a user's install ([ADR-0014](../decisions/ADR-0014-xtask-workspace-for-dev-tooling.md)).

### Provenance

For each key, the wrapper tracks which layer supplied the winning value. This is what makes "why is it doing that?" answerable in one command rather than by bisecting files. `config` reports it.

## Composing the child's settings

The user authors wrapper pieces and a profile; the wrapper composes them into an entry in the [composed-settings store](./xdg-storage.md#composed-settings-entries), supplied as a native `--settings` layer under [ADR-0028](../decisions/ADR-0028-pass-composed-settings-with-the-native-flag.md). That entry is one layer among the child's several, not the child's whole settings — [where composition stops](#where-composition-stops) says which.

### Inputs

Pieces are partial settings documents in the child's own format, JSON, under `settings/` in the config base. Each is a fragment: a piece that only sets one key contains only that key. Pieces are read-only to the wrapper.

Profiles are the named sets. One YAML file per profile under `profiles/`, whose sole required field is an ordered, non-empty list of piece names:

```yaml
# profiles/work.yaml
layers:
  - base
  - work-permissions
  - verbose-logging
```

Order is significant and later wins. Unknown fields in a profile are rejected. An empty list is rejected. A referenced piece that does not exist is an error naming both the profile and the resolved path it looked for.

One name, four layers, everywhere the concept appears ([ADR-0050](../decisions/ADR-0050-name-the-profile-surface-once.md)).

### Selecting the active profile

The flag is `--profile <name>`; the configuration key and its environment spelling are [`default_profile`](#keys). One rung sits between the project file and user configuration: the selected account's own binding ([ADR-0096](../decisions/ADR-0096-bind-a-profile-to-an-account.md)), which [accounts](./accounts.md#the-bound-profile) owns. So a profile resolves from, in order, the flag, the environment, the project file, the selected account's binding, and user configuration.

The binding is below the project file because a project file is a deliberate per-tree override, and above user configuration because a binding names one account's settings where the user key names everyone's.

With nothing set and no `--profile`, no name is resolved. That remains a valid loader and report state: `config` may report no active profile at exit `0`. A launch is not ready, however, and refuses as `Config` until `--profile` or `default_profile` resolves a name ([ADR-0090](../decisions/ADR-0090-require-account-and-profile-before-child-launch.md)).

A name that is resolved must exist. `profiles/<name>.yaml` missing is `NoInput`, whichever layer supplied the name: a profile the user asked for and did not get would launch the child under settings the user believes are something else, which is the failure every strict rule on this page exists to prevent. See [exit codes](./exit-codes.md#exit-regimes-by-verb).

A profile name becomes a path component in the composed-settings store, so it must satisfy [the identifier rules](./xdg-storage.md#identifiers); a name that does not exits `Usage`, whichever layer supplied it.

### Merge semantics

Pieces are folded left to right into one document:

| Node type             | Rule                                                                      |
| --------------------- | ------------------------------------------------------------------------- |
| Object                | Merged key by key, recursively                                            |
| Scalar                | Last writer wins                                                          |
| Array                 | Replace by default                                                        |
| Array, `concat`       | Elements appended in layer order                                          |
| Array, `merge-by-key` | Elements matched on a named field and merged; unmatched elements appended |

Array strategy is the one genuinely contested decision. Replace is the default because it is predictable: what the last piece says is what you get. Concatenation is what you want for additive lists — extra permitted paths, extra tools — and it is opt-in per key through a strategy table in the profile, because a global concat setting is wrong for roughly half of any real settings file.

Worth knowing before it surprises a piece author: the child merges arrays the other way. Where one array-valued setting appears in several of the child's own scopes, the child concatenates and de-duplicates rather than replacing. The wrapper's pieces are not the child's scopes, and predictability wins inside the wrapper — but a `permissions.allow` split across two pieces behaves differently from the same list split across two of the child's files. The child's rule is tracked as [`child-settings-scope-precedence`](./research-tracking.yaml).

#### Declaring a strategy

Exceptions are declared beside the layer list, keyed by [RFC 6901](https://www.rfc-editor.org/rfc/rfc6901.html) JSON Pointer:

```yaml
# profiles/work.yaml
layers:
  - base
  - work-permissions

array_strategies:
  "/permissions/allow": { strategy: concat }
  "/hooks/PreToolUse": { strategy: merge-by-key, key: matcher }
```

A pointer, not a dotted path: the child renders nested settings keys dotted — `permissions.allow`, `sandbox.filesystem.allowWrite` — so a dotted strategy key stops being addressable the moment a settings key contains a dot, while RFC 6901's `~0` and `~1` escapes leave nothing ambiguous. It keeps the same standards family as the merge model already cited below.

| Rule                       | Detail                                                                 |
| -------------------------- | ---------------------------------------------------------------------- |
| `replace`                  | Never written — it is what an unlisted array does                      |
| `concat`                   | Takes no other field                                                   |
| `merge-by-key`             | Requires exactly one non-empty `key`; elements are objects carrying it |
| Non-canonical pointer      | `DataFormat`                                                           |
| Pointer naming a non-array | `DataFormat`                                                           |
| Pointer matching no key    | `DataFormat`                                                           |
| Duplicate merge-key values | `DataFormat`                                                           |

Matched objects merge recursively; unmatched elements append in layer order. A strategy that silently applies to nothing leaves a profile promising settings the composed document does not carry, which is why the third row is an error rather than a warning.

A type conflict — one piece making a key an object and another a string — is an error, not a silent overwrite. It names the key path, both pieces, and both types. The rule holds at every depth, including inside a pair of `merge-by-key` elements that matched: the reported pointer reaches the field that conflicted, and the array stays one entry in the provenance sidecar. Two values that are merely different types, neither of them an object, remain an ordinary override wherever they meet.

Merging is deterministic: the same inputs produce byte-identical output, with object keys in a stable order. It is what lets an entry be named by its inputs at all, and it makes diffs useful.

### Provenance sidecar

Alongside the composed settings, the wrapper writes a sidecar. It exists because nothing downstream answers the question: the child reports which settings sources it loaded, not which one supplied a given key.

| Field          | Contents                                                                   |
| -------------- | -------------------------------------------------------------------------- |
| `profile`      | The profile name                                                           |
| `profile_path` | Its resolved path                                                          |
| `digest`       | The full [input digest](./xdg-storage.md#composed-settings-entries)        |
| `pieces`       | The pieces in profile order, each `name` and resolved `path`               |
| `keys`         | One entry per leaf key, addressed by [JSON Pointer](#declaring-a-strategy) |

Each `keys` entry names the `piece` that supplied the winning value. Where more than one piece touched the key it also carries the ordered chain — `overrode` for a scalar, `contributors` and `strategy` for a merged array — because "the setting is wrong" and "the setting is wrong because this piece overrode that one" are different answers and only the second is useful. A single-contributor key stays one line: an optional field absent is omitted, never null.

```json
{
  "profile": "work",
  "profile_path": "/home/u/.config/claude-session-rs/profiles/work.yaml",
  "digest": "8f2a91c3d40b…",
  "pieces": [
    { "name": "base", "path": "/home/u/.config/claude-session-rs/settings/base.json" },
    { "name": "work-permissions", "path": "/home/u/.config/claude-session-rs/settings/work-permissions.json" }
  ],
  "keys": {
    "/model": { "piece": "base" },
    "/env/HTTPS_PROXY": { "piece": "work-permissions", "overrode": ["base"] },
    "/permissions/allow": { "piece": "work-permissions", "strategy": "concat", "contributors": ["base", "work-permissions"] }
  }
}
```

A path is an OS byte string and JSON is not ([process runtime](./process-runtime.md#child-argument-vector)). `path` and `profile_path` carry the lossy display form; a `path_b64` sibling appears beside either only where that form is not byte-exact.

There is no version field. The digest preimage's domain tag already versions the format: changing the sidecar's shape bumps the tag, which renames every entry, which makes an old sidecar unreachable rather than misread. A field with one possible value discriminates nothing ([ADR-0051](../decisions/ADR-0051-let-every-surface-element-discriminate.md)).

Two input sets that happen to render identical settings still get separate entries, because provenance is a function of the inputs rather than of the output. `config` surfaces all of it.

### Generation

Generation resolves the profile, loads the pieces, computes [the entry key](./xdg-storage.md#composed-settings-entries), and stops if that entry already exists and its sidecar agrees. Otherwise it merges, validates, and writes the settings and its provenance atomically.

Existence is the whole freshness answer. Because the key covers the profile and every referenced piece by content, editing a piece names a different entry — stronger than the modification-time comparison it replaces, and without the "an edit that appears to do nothing" bug that comparison existed to catch.

### Where composition stops

Composition ends at one file. The wrapper hands it over as `--settings <absolute path>`, prepended to the child's argument vector ([process runtime](./process-runtime.md#child-argument-vector)), and resolves nothing beyond it.

The child places that document in its command-line-arguments tier: above its local, shared-project, and user settings, and below managed settings, which the composed document cannot override. The wrapper passes no `--setting-sources`, so the working directory's own `.claude/settings*.json` still load beneath the composed layer. Neither is a defect to repair — the child owns its resolution — but both are invisible from inside the wrapper, so `config` reports the composed entry and never claims it is what the child will run.

`CLAUDE_CONFIG_DIR` does not carry the composed file, for two reasons. It relocates the child's whole tree — settings, history, plugins, and the saved login — so making it per-profile would split the one credential store [accounts](./accounts.md) exists to keep whole, already rejected in [ADR-0064](../decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md). And it would land the composed document in the child's lowest tier, where a repository's checked-in settings would outrank the profile the user explicitly asked for. The first reason is about credentials; the second is about the feature not working.

A user's own `--settings` still replaces the wrapper's by last occurrence, accepted rather than repaired ([ADR-0047](../decisions/ADR-0047-let-a-user-settings-flag-override-the-group-layer.md)).

### Validation

Validation stops where ownership stops. The wrapper validates the structure it owns, strictly and by parsing into a type. It knows nothing about the child's settings keys — not their names, not their types, not whether they exist — because the child owns that schema, validates it, and reports its own errors. A wrapper that also modelled it would be a second, always-lagging copy of something already handled, and the first thing such a copy does is reject a setting the child accepts.

That is the same asymmetry the wrapper's own configuration states from the other side: strict about what we own, permissive about what we forward. It is the passthrough contract applied to configuration.

What the wrapper owns, and therefore parses and validates hard, is the profile document, the strategy table and its applicability, cross-piece type conflicts, each piece being well-formed JSON with an object root, and the composed document's own well-formedness. Each of those is a property of the composition itself rather than of what any key means. A type conflict is refused not because the wrapper knows what the key is for, but because two pieces disagreeing about whether it is a container leaves no defined merge.

Beyond that boundary the wrapper's only obligation is to generate the composed document consistently. Every key a piece supplies reaches the child exactly as written, whatever it is called. Nothing is warned about, nothing is filtered, and nothing is reported.

### Child-owned account state

Trust, project history, and other native state remain child-owned in the shared account `config/`. The wrapper neither seeds nor synchronizes them.

Onboarding is the one exception, and it is one key. A successful [login](./accounts.md#logging-in) records that the child's first-run setup is done, because the wrapper created the directory whose newness makes the child ask ([ADR-0098](../decisions/ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md)). Nothing else in that file is written, and the per-workspace trust prompt still fires.

## Commands

Two verbs, neither with subcommands ([ADR-0049](../decisions/ADR-0049-collapse-config-inspection-into-one-verb.md), [ADR-0051](../decisions/ADR-0051-let-every-surface-element-discriminate.md)).

| Command   | Reports                                                                                                                                                                                                                       |
| --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `config`  | The resolved wrapper configuration with per-key provenance; which files were consulted and which existed; the active profile, its resolved pieces, and the resolved entry path with whether it exists; and every defect found |
| `profile` | The profiles available to `--profile`                                                                                                                                                                                         |

Both accept `--json`, and both write data to standard output and diagnostics to standard error; see [logging and output](./logging-and-output.md). `config` reports one profile's ordered layers and resolved paths when given `--profile <name>`.

`--profile` is declared on the two invocations that act on it — the bare launch, which composes that profile's settings for the child, and `config`, which resolves and reports it. It is not a global flag, because on every other verb it would name a value nothing reads ([ADR-0051](../decisions/ADR-0051-let-every-surface-element-discriminate.md)).

`config` validates, so it is an assertion verb: a structural defect or type conflict exits with that defect's code, while an entry not yet written stays advisory at `0`. The exact table is in [exit codes](./exit-codes.md#exit-regimes-by-verb).

The checks it runs are the config-scoped subset of [the one probe catalog](./doctor.md#the-catalog) `doctor` runs in full, so the two cannot disagree and quote one remediation wording ([ADR-0018](../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)). `doctor` reports health and never renders configuration.

## Further reading

- [`figment`](https://docs.rs/figment/)
- [`serde_json`](https://docs.rs/serde_json/)
- [RFC 7396: JSON Merge Patch](https://www.rfc-editor.org/rfc/rfc7396.html)
