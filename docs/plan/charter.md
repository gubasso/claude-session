# Delivery charter

## Purpose

Deliver a self-contained Rust wrapper that adds account and profile isolation while behaving as stock `claude` unless the user explicitly invokes a wrapper feature.

## Pillars

- Preserve native passthrough byte-for-byte and status-for-status.
- Place and secure every artifact through XDG ownership.
- Keep build, operation, and governing knowledge self-contained in this repository.

## No-gos

- Do not parse or rewrite native child grammar.
- Do not write directly under `$HOME` or depend on personal or external local trees.
- Do not add speculative CLI flags, verbs, configuration keys, or abstractions.
- Do not read, copy, refresh, or fingerprint child-owned credentials.

## Appetite

The appetite unit is implementation sessions. One session is one discrete implementation run ending with the slice's relevant tests and a handoff. Appetites are budgets. Cut scope from the end of `In scope`, never from core correctness, security, or tests.
