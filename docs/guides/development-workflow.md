# Development workflow

Ordered procedures for working on `claude-session`. Each is a task with prerequisites and a verification step. Rules are linked, not restated — if a step and a linked document disagree, the linked document wins and this page is wrong.

## Prerequisites

The project pins its toolchain and provides the surrounding tools through a Nix development shell. Do not depend on whatever happens to be installed on the host.

```bash
direnv allow      # loads the shell automatically on entering the directory
nix develop       # or enter it explicitly
```

Install the git hooks once:

```bash
just hooks-install
```

Verify the baseline before changing anything:

```bash
pre-commit run --all-files
```

A red baseline means an unrelated problem exists. Fix or report it first, so that later failures are attributable to your work.

## Pick up the next piece of queued work

Implementation work is queued under `.implementation-plans/`. Status lives in YAML, not in the directory layout: nothing moves on disk as work progresses.

1. Read `.implementation-plans/queue-plans.yaml`. It lists every plan, its status, and its dependencies.
2. Choose a plan whose status is `todo` and whose dependencies are all `done`.
3. Read that plan's `README.md` for the problem statement and the round breakdown.
4. Read the plan's `queue-rounds.yaml` and find the first round with status `todo`.
5. Read that round file end to end **before** writing code. It names its scope, what is deliberately out of scope, and its acceptance criteria.
6. Read the specifications the round cites. They are the contract the round implements against.
7. Set the round's status to `doing`, implement **only that round**, then set it to `done`.

**One round per session.** Rounds are sized to be a self-contained unit of work; running two together makes the result unreviewable and makes a failure hard to localize.

**A round file is not a specification.** It describes work. The durable contracts live under `docs/`. If a round contradicts a specification, the specification wins — see the reconciliation procedure below.

## Add a wrapper command

Adding a verb touches exactly four files, in this order. The four-edit rule and its rationale are in [the architecture](../explanation/architecture.md).

1. `src/cli/<verb>.rs` — the arguments struct. Parser declarations only, no logic.
2. `src/cli.rs` — the new variant on the commands enum.
3. `src/commands/<verb>.rs` — the handler, a free function over the context and the arguments.
4. `src/commands/dispatch.rs` — the match arm routing the variant to the handler.

Then:

- Check the flag names against [the CLI surface](./../reference/cli-surface.md). **Every flag the wrapper claims is a flag the child can no longer receive** — that is a passthrough-contract change and needs a decision record, not just a table edit.
- Send results to standard output and everything else to standard error, through the single output writer. See [logging and output](./../reference/logging-and-output.md).
- Declare the verb's own `--json` if it produces data worth consuming from a script.
- Add an integration test file `tests/cmd_<verb>.rs` and a help snapshot.

Verify:

```bash
just lint
just test-unit
just test-integration
```

## Add a dependency

1. Check [dependencies](./../reference/dependencies.md). If the crate is on the reviewed list, proceed. If it is on the ruled-out list, use the named alternative. If it is on neither, assess maintenance, licence, and transitive weight, and record that assessment in your change.
2. Add it with `cargo add`, never by editing the dependency table and never by writing a version string:

   ```bash
   cargo add <crate> --features <features>
   ```

   The tool resolves the graph and updates the lockfile in one step. Hand-editing skips resolution, leaving the manifest and lockfile disagreeing.
3. Commit `Cargo.lock`.
4. If the choice is hard to reverse — a runtime, an async framework, a serialization format — write a decision record. If it merely implements something already decided, it does not need one.

Verify:

```bash
just deny
just audit
```

`cargo machete` runs at push and will fail on a dependency added but not yet used. Add the crate in the same change that uses it.

## Write a test

Read [the testing strategy](../explanation/testing-strategy.md) once, then use [testing and quality](./../reference/testing-and-quality.md) as the lookup.

1. Choose the kind. Pure logic gets a unit test beside the code. A seam between components gets an integration test in `tests/`. Anything needing the real `claude` is end-to-end and belongs in continuous integration only.
2. Make it hermetic: a fresh temporary directory, a **cleared** child environment rather than an inherited one, every base directory variable pointed inside the temporary directory, no network, no real clock.
3. Never mutate the test process's own environment or working directory. Both are shared across the parallel runner and will corrupt unrelated tests.
4. For anything involving the child, use the recording stub rather than the real binary.
5. If your change touches a contract in the mandatory-test table, extend that test rather than adding a parallel one.

Verify:

```bash
just test-unit          # the commit lane
just test-integration   # the push lane
```

Each names its profile explicitly. A `nextest` profile is inert unless `--profile` is passed, so an invocation that omits it silently runs everything with the wrong settings.

## Record a decision

Write a decision record when a choice is significant and hard to reverse — anything touching the passthrough contract, the storage layout, the CLI surface, or a dependency that would be painful to back out.

1. Copy `docs/decisions/template.md` to `docs/decisions/NNNN-short-title.md`, taking the next free number.
2. Title it after **the choice**, not the task. "Spawn and wait rather than exec", not "process work".
3. Keep the filled body at or under 350 words, in the template's five sections. The cap is a splitting signal: if it will not fit, you are recording more than one decision. Worked detail belongs in the reference page, and the record links to it.
4. List the alternatives you seriously considered, with their bad consequences as well as good ones. A record with one option is a record of nothing.
5. Status is `Accepted` for a binding decision with no code yet, `Implemented` once code enacts it.
6. Link the record from the document that owns the detail, and link that document from the record.

**Never delete a record.** A decision that stops being true is `Superseded` by a new one, with a forward link. One whose context evaporated with no successor is `Deprecated`. One partly changed keeps its status and gains an `Amended by ADR-NNNN` line. A rejected option worth not re-debating stays as a `Rejected` record.

## Reconcile a contradiction

When a round file, a code comment, or another document disagrees with a specification:

1. **The specification wins by default.** Edit the other side to match.
2. **If the other side is actually right**, update the specification _and_ the decision record carrying that decision. Changing a specification without its record leaves the rationale describing a design that no longer exists.
3. **If it is a genuinely open question**, write a new record with status `Proposed`. Two live claims in one repository is the drift this project's whole documentation model exists to prevent.

Do not leave a contradiction unresolved on the grounds that a plan is only a plan.

## Work with drafts

`.draft/` is gitignored. It is a workshop for research notes and scratch material.

Promotion out of it is a **rewrite into the owning document**, not a move. A draft is written for you; a specification is written for the next reader. Once a draft's substance has been rewritten, delete the draft — a promoted draft kept "just in case" becomes a second, stale source of truth.

## Commit

Commits follow Conventional Commits, checked by a hook:

- Type and optional scope, then a lowercase description, with **no trailing period**.
- **Every line, including the subject, at or under 72 characters.** The subject limit is stricter than most conventions; the hook will tell you.
- Tune `committed.toml` rather than the hook configuration if the rules need changing.

```text
docs: add wrapper model and process runtime specs
```

## Branch, review, and release

Two long-lived branches, plus short-lived feature branches.

| Branch                       | Role                                                                                           |
| ---------------------------- | ---------------------------------------------------------------------------------------------- |
| `develop`                    | The integration branch and the **release trigger**. Release automation watches it.             |
| `master`                     | The release branch — a mirror of the latest published version. **No human ever writes to it.** |
| `feat/…`, `fix/…`, `chore/…` | Short-lived, branched off `develop`, merged back through a reviewed PR.                        |

The flow is one-way:

```text
feat/*  ──PR──▶  develop  ──release──▶  (tag vX.Y.Z)  ──CI promote──▶  master
```

Keep a feature branch **linear by rebasing** onto `develop` rather than merging `develop` into it; a branch full of back-merges is unreviewable as a diff. Merge only through a reviewed PR with green CI.

`master` is written by CI, which fast-forwards it after a successful release. A human pushing to `master` breaks the invariant that it mirrors exactly what was published — which is the only reason it is worth having a second branch at all. Protect it at the forge rather than relying on discipline.

Releases are cut by automation from `develop`; the publishing procedure and its credentials are in `PUBLISHING.md`.

## Dependency and security baseline

Local scanning runs in the gate — secret scans, advisories, and licence checks all sit in `pre-commit`. It reports; it does not rewrite. Two things follow.

**Upgrades are authored.** Run `cargo update` or edit a pinned version, target `develop`, and let the gate decide whether it lands, exactly as for any other change. `release-plz` opens the release pull request and is the only automation that opens one ([ADR-0023](../decisions/0023-only-release-automation-opens-pull-requests.md)).

**A workflow's action pins are tags, and a tag is mutable.** An unchanged `uses:` line does not mean unchanged code: the referenced tag moves when its maintainer moves it, so CI can change behaviour with no commit here to explain it. Nothing in the gate reads `.github/`, so currency is re-checked on a cadence — [research tracking](../reference/research-tracking.yaml), `pinned-action-currency`.

**Branch protection** is the one part of this baseline that lives at the forge, and it is not configured yet. The `master` invariant above is a forge setting, not a convention; configure it so CI is the only writer.

## Before proposing a change

```bash
pre-commit run --all-files
```

This is the single command that reproduces the project's verdict. Fix everything it reports; do not bypass a hook. Expect the markdown formatter to unwrap paragraphs to one physical line and the linter to renumber ordered lists — accept those rewrites and re-run until clean.

If a hook is wrong, fix its configuration in a separate change with a reason. Do not add a suppression to get past it.
