# Testing strategy

Testing a wrapper is unlike testing an ordinary CLI. The interesting behaviour is not what the program computes; it is what it hands to another process, what it does to that process's environment, and how faithfully it relays the result. This page explains the approach. The tool list, the test lanes, and the gate configuration are in [testing and quality](../reference/testing-and-quality.md).

## The shape of the suite

**Unit tests** live beside the code they test, in the same file. They cover pure functions: the argv pre-split, identifier validation, the settings merge, exit-code mapping. These are where most of the assertions should be, because most of the risky logic in this program is pure and can be tested without a process, a filesystem, or a clock.

**Integration tests** live in the test directory, one file per wrapper verb plus one per cross-cutting contract, and they run the compiled binary. They test the _seam_ between components. That is the defining property — not "touches real things". A test that runs the real binary against a stub child is an integration test; so is one that exercises the configuration loader against a temporary directory tree.

**End-to-end tests** run the whole thing against the real `claude`. They belong in continuous integration only, never in a local commit or push hook: they are slow, they depend on host state, and they need credentials. A wrapper's test suite that requires the wrapped tool to be installed is a test suite most contributors cannot run.

The pyramid's proportions matter more than its layers. If the fast unit layer is thin and the slow binary-invoking layer is thick, the suite will be slow, flaky, and bad at localizing failures — three symptoms of the same cause.

## The seam that makes this testable

A program that calls the process-spawning API directly from its command handler cannot be tested without spawning real processes. This project avoids that by making spawning a **port**: the handler depends on a spawner trait, and the real implementation lives in the adapter layer. A test substitutes a fake that records what it was asked to do and returns whatever outcome the test needs.

This is the single most valuable structural decision in the codebase from a testing standpoint, and it is why [the architecture](./architecture.md) insists that adapters are the only place touching the outside world. The same applies to the filesystem and the clock: a service that reads the current time from a global cannot be tested for time-dependent behaviour, while one that takes a clock can.

The seam does not remove the need for real-process tests. It changes what they are for: proving the real adapter works, once, rather than being the only way to test anything.

## The recording child

For tests that do want a real process, the wrapper spawns a **stub binary** placed on the search path ahead of anything else. The stub does one job: record the arguments it received, the environment it was given, and the working directory it started in, then exit with whatever status the test asked for.

This gives exact, mechanical assertions about the passthrough contract. Not "the command looked right" but "the child received these arguments, in this order, with these bytes." The stub also makes it easy to test the failure modes that are awkward with a real child: an immediate non-zero exit, death by signal, a binary that exists but is not executable, a binary that is not there at all.

## Hermetic, without exception

Every test that touches the environment or the filesystem must be **hermetic**: it observes nothing from the developer's machine and leaves nothing behind.

- A fresh temporary directory per test, removed afterwards. Never a shared fixture directory, because a shared directory makes tests order-dependent and makes parallel execution a source of flakes.
- The child's environment is **cleared and then explicitly populated**, rather than inherited and patched. A test that inherits the developer's environment passes on the developer's machine and fails in continuous integration, or worse, passes in both for the wrong reason.
- All base directory variables are pointed inside the temporary directory. A test must never be able to read or write the real user's configuration, state, or credentials. This is not merely tidiness: the program under test writes credential files.
- No network, and no dependence on the real clock. Where a timestamp is observable, it is injected.
- No mutation of the test process's own global state — no setting environment variables in the test process, no changing the working directory. Both are shared across a parallel test runner and will corrupt unrelated tests in ways that reproduce roughly one run in twenty.

Hermeticity is stated as a hard rule rather than a preference because the failure it prevents is the most expensive kind: an intermittent failure that is not reproducible, and that trains the team to re-run the suite instead of reading it.

## What must have a test

Some contracts in this project are load-bearing enough that they are specified together with the test that locks them down. If the test is missing, the contract is decorative.

**Golden argv.** For a table of representative command lines — a bare passthrough, a leading child flag, an empty argument, a non-UTF-8 argument, arguments around `--`, a wrapper flag before a child flag — assert the exact argument vector the child received. This is the passthrough contract, and it is the thing most likely to break silently during an unrelated refactor.

**The exit-code matrix.** Every variant of the error type is asserted against its documented code, with no catch-all arm anywhere in the mapping. Written this way, adding an error variant without assigning it a code fails the build rather than silently returning a generic failure. See [exit codes](../reference/exit-codes.md).

**Child status fidelity.** A stub exiting with a given status produces that status from the wrapper. A stub killed by a signal produces the signal-derived status. These are separate assertions because they take different paths through the mapping.

**The `--` sentinel.** A wrapper flag spelled after `--` reaches the child as an argument and is not interpreted. This is one test and it prevents a whole class of surprising behaviour.

**The recursion guard.** With the wrapper itself resolvable as the child, the wrapper refuses rather than recursing. Worth testing both guard paths — the marker variable and the file-identity self-check — since either alone has a hole.

**Isolation.** The stub child observes the injected configuration directory, and observes that the wrapper's internal variables have been scrubbed from its environment.

**Filesystem security.** A session directory that is a symbolic link is rejected. A directory with over-permissive mode is corrected or refused. These tests are the reason the security posture in [session isolation](./session-isolation.md) is real rather than aspirational.

## Anti-patterns this project rejects

Named, so that review can point at the name rather than re-arguing the case:

**Testing the library, not the code.** Asserting that the process-spawning API spawns a process, or that the JSON parser parses JSON. These tests pass forever and catch nothing.

**The shared fixture directory.** One directory reused across tests. Introduces order dependence, breaks parallelism, and produces failures that only appear in full-suite runs.

**Global environment mutation.** Setting a variable or changing the working directory in the test process. Shared with every other test in the binary.

**Brittle exact-match assertions on prose.** Asserting the full text of an error message. The message will be improved, the test will fail, and the test will be updated without thought — which means it was never checking anything. Assert the stable machine-readable error kind and the exit code; assert that the message _contains_ the offending path.

**Snapshot abuse.** Snapshots are excellent for help output and structured reports, where the whole shape is the contract. They are poor for anything with an embedded path, timestamp, or hash, and they are actively harmful when a failing snapshot is accepted reflexively. A snapshot that nobody reads before accepting is a test that has been turned off.

**Untested help.** `--help` is user-facing output that changes whenever a flag changes. It should be snapshotted precisely because it is generated.

**The monolithic integration test.** One test that exercises everything, so that any failure means reading a hundred lines of setup to find out what broke. One file per verb, one contract per test.

**Depending on the real wrapped tool.** Any test that requires `claude` to be installed and authenticated. It belongs in the end-to-end lane or nowhere.

## Further reading

- [Testing — Command Line Applications in Rust](https://rust-cli.github.io/book/tutorial/testing.html)
- [`assert_cmd`](https://docs.rs/assert_cmd/)
- [`insta`](https://insta.rs/docs/)
- [`trycmd`](https://docs.rs/trycmd/)
