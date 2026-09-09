# Security policy

## Reporting a vulnerability

Report a vulnerability privately through [GitHub private vulnerability reporting](https://github.com/gubasso/claude-session/security/advisories/new). Do not open a public issue for one.

Include what you have: a description of the problem, the steps that reproduce it, the version you saw it on, and any mitigation you know of.

## What happens next

One maintainer reads the report. Handling is best effort, so expect a first reply in weeks rather than days, and expect it to say whether the report is accepted or declined.

An accepted report becomes a GitHub security advisory on this repository, which is also how a fix is announced. A published advisory reaches the [RustSec advisory database](https://rustsec.org), so `cargo audit` sees it.

## Supported versions

Only the latest published version is supported. This project is `0.x`, a fix ships as a new version rather than as a patch to an older one, and a yank contains a bad version rather than recovering it. [The release workflow](https://github.com/gubasso/claude-session/blob/master/docs/reference/release-workflow.md) owns that policy.

## Dependency advisories

A CVE in a dependency is not by itself a vulnerability in this wrapper. CI already runs `cargo audit` and `cargo deny` on every change, so a report that names a version and an advisory id, with no path showing the problem is reachable from this code, tells nobody anything new. Show the reachable path or the proof of concept.

## What this project cannot do

This is a wrapper around the `claude` CLI. A vulnerability in `claude` itself, in the Anthropic API, or in a dependency belongs to whoever owns that code. Report it there. This policy covers the wrapper.
