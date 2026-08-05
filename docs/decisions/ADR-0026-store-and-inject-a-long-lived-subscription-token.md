# ADR-0026: Store and inject a long-lived subscription token

## Context and Problem Statement

Subscription-only automation needs durable authentication without copying or interpreting the child's saved login. The wrapper must own token storage, injection, reporting, rotation, and removal without implementing OAuth.

## Considered Options

- Store a private token file and inject it per child run.
- Parse or reuse the child's credential.
- Call an undocumented OAuth endpoint.
- Manage API-billing credentials.
- Make a linked keyring the default.

## Decision Outcome

Chosen option: private file plus per-run injection — token-mode accounts store an OAuth token in a `0600` wrapper-owned file under the `0700` account directory and inject it as `CLAUDE_CODE_OAUTH_TOKEN` together with the account's `CLAUDE_CONFIG_DIR`.

The file is written atomically after ingestion under [ADR-0027](./ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md). Mode metadata records `mode`, `recorded_at`, and `sha256[..8]`; it never infers lifetime from a token prefix. Age and estimated expiry use the recorded or corrected mint time and label expiry as an estimate.

## Consequences

- Good: each account has persistent, subscription-only, per-process authentication.
- Good: reports can distinguish rotations without exposing the token.
- Bad: replacement must be staged and verified before the old token is replaced.
- Bad: account removal deletes local use but cannot revoke the upstream token.
- Bad: the wrapper must protect token material across every output and error surface.

The wrapper never parses or fingerprints the child credential, speaks an OAuth endpoint, or manages API-billing credentials. A helper may replace file retrieval only through [ADR-0029](./ADR-0029-use-a-credential-helper-process-boundary.md).

## Status

Accepted

Amended by [ADR-0067](./ADR-0067-commit-a-token-rotation-with-the-metadata-rename.md) — verification before replacement stands; the staging it names is the atomic-write temporary, not an artifact of its own.
