# ADR-0128: Tell a saved login by its key

## Context and Problem Statement

The child keeps its claude.ai login and every MCP server sign-in in one file, `.credentials.json`. The wrapper took a file at that path as a saved login. An MCP sign-in in a token account therefore raised a false shadowing warning. It made a login-mode account usable with no login, and a mode switch deleted sign-ins that superseded nothing.

## Considered Options

- Read the names of the file's top-level keys, and take a saved login to mean that `claudeAiOauth` holds an object.
- Keep the presence test, and word every surface as "a credential file exists".
- Rewrite the file without `claudeAiOauth` when a switch retires the login, so the MCP sign-ins survive.

## Decision Outcome

Chosen option: read the key names, because only the file's contents tell the two halves apart.

The wrapper opens the file read-only, never through a link, and reads it only up to a fixed bound. It skips every value as the parser passes it. It keeps, compares, prints, and fingerprints no value, and it never writes the file. A missing, linked, foreign, oversized, or malformed file holds no saved login, as the child also sees it.

A switch to token mode unlinks the file only when it holds a saved login. A file that holds both halves goes whole, and the report names the MCP sign-ins that went with it. A rewrite would race the child's own writer, and the wrapper does not author child credential files.

## Consequences

- Good: the warning, `account status`, launch usability, the login commit, and retirement each answer their own question.
- Good: an MCP sign-in no longer disappears on a switch that supersedes no login.
- Bad: the wrapper now depends on two key names of the child, `claudeAiOauth` and `mcpOAuth`, which can move without notice.
- Bad: a switch away from a saved login still costs the MCP sign-ins in the same file.

## Status

Implemented

Amends [ADR-0025](./ADR-0025-share-one-native-login-per-account.md) and [ADR-0101](./ADR-0101-retire-the-credential-a-mode-switch-supersedes.md). Enacted in [the saved-login probe](../../src/services/account/saved_login.rs). Shaped by [041](../plan/slices/041-saved-login-presence/README.md).
