# Testing strategy

Testing a wrapper is unlike testing an ordinary CLI. The interesting behaviour is not what the program computes; it is what it hands to another process, what it does to that process's environment, and how faithfully it relays the result. This page explains the approach and the reasoning behind it. Every exact rule — the lanes and what each admits, the hermetic requirements, the recording stub's format, and the mandatory-test table — is in [testing and quality](../reference/testing-and-quality.md).

## The shape of the suite

The three lanes are a pyramid, and its proportions matter more than its layers. Most of the risky logic in this program is pure — the argv pre-split, identifier validation, the settings merge, exit-code mapping — and can be tested without a process, a filesystem, or a clock. That is where most assertions belong.

If the fast unit layer is thin and the slow binary-invoking layer is thick, the suite will be slow, flaky, and bad at localizing failures. Those are three symptoms of one cause, and the cause is testing pure logic through a process boundary because the seam to test it directly was never built.

The pressure runs the other way too. A wrapper's whole contract is observable only at the process boundary, so the integration layer cannot be thin either: a suite that proves the merge function is correct and never checks what reached the child has tested everything except the product.

## The seam that makes this testable

A program that calls the process-spawning API directly from its command handler cannot be tested without spawning real processes. This project avoids that by making spawning a port: the handler depends on a spawner trait, and the real implementation lives in the adapter layer. A test substitutes a fake that records what it was asked to do and returns whatever outcome the test needs.

This is the single most valuable structural decision in the codebase from a testing standpoint, and it is why [the architecture](./architecture.md) insists that adapters are the only place touching the outside world. The same applies to the filesystem and the clock: a service that reads the current time from a global cannot be tested for time-dependent behaviour, while one that takes a clock can.

The seam does not remove the need for real-process tests. It changes what they are for: proving the real adapter works, once, rather than being the only way to test anything. Those remaining tests run against a stub child rather than `claude`, which is what makes the awkward cases — an immediate non-zero exit, death by a chosen signal, a binary that exists but is not executable — ordinary to write.

## Anti-patterns this project rejects

Named, so that review can point at the name rather than re-arguing the case:

Testing the library, not the code. Asserting that the process-spawning API spawns a process, or that the JSON parser parses JSON. These tests pass forever and catch nothing.

The shared fixture directory. One directory reused across tests. Introduces order dependence, breaks parallelism, and produces failures that only appear in full-suite runs.

Global environment mutation. Setting a variable or changing the working directory in the test process. Shared with every other test in the binary.

Brittle exact-match assertions on prose. Asserting the full text of an error message. The message will be improved, the test will fail, and the test will be updated without thought — which means it was never checking anything. Assert the stable machine-readable error kind and the exit code; assert that the message contains the offending path.

Snapshot abuse. Snapshots are excellent for help output and structured reports, where the whole shape is the contract. They are poor for anything with an embedded path, timestamp, or hash, and they are actively harmful when a failing snapshot is accepted reflexively. A snapshot that nobody reads before accepting is a test that has been turned off.

Untested help. `--help` is user-facing output that changes whenever a flag changes. It should be snapshotted precisely because it is generated.

The monolithic integration test. One test that exercises everything, so that any failure means reading a hundred lines of setup to find out what broke. One file per verb, one contract per test.

Depending on the real wrapped tool. Any test that requires `claude` to be installed and authenticated. It belongs in the end-to-end lane or nowhere.

Laundering bytes through a string type. Anywhere in the harness — the stub, an assertion, a snapshot. A non-UTF-8 argument is legal on Unix and the wrapper promises to carry it, so a conversion inside the test destroys the one case worth testing and the test passes against a wrapper that had already broken it.

## Further reading

- [Testing — Command Line Applications in Rust](https://rust-cli.github.io/book/tutorial/testing.html)
- [`assert_cmd`](https://docs.rs/assert_cmd/)
- [`insta`](https://insta.rs/docs/)
- [`trycmd`](https://docs.rs/trycmd/)
