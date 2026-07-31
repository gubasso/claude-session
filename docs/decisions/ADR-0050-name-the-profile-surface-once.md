# ADR-0050: Name the profile surface once

## Context and Problem Statement

One concept carried two names. The file on disk was a **manifest** (`manifests/<profile>.yaml`, `manifest.rs`, `manifest.example.yaml`), while the flag, the verb, and the configuration key called it a **profile** — forcing [configuration](../reference/configuration.md) to open its own definition with "**Manifests** are profiles." A reader who learns one name cannot predict the other, and every error message, error path, and generated artifact has to pick a side.

## Considered Options

- **Keep both**, with "manifest" for the artifact and "profile" for the selector.
- **Standardize on `manifest`**, renaming the flag and verb.
- **Standardize on `profile`**, renaming the directory, the type, and the generated example.

## Decision Outcome

Chosen option: **standardize on `profile`**, because it is the name in the surface the user types, and a rename of the flag would spend the passthrough contract's append-only budget ([ADR-0044](./ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md)) to buy nothing.

One name at every layer: the directory `profiles/`, the file `profiles/<name>.yaml`, the flag `--profile <name>`, the verb `profile`, the environment variable `CLAUDE_SESSION_DEFAULT_PROFILE`, the configuration key `default_profile`, and the generated `examples/profile.example.yaml`. The word "manifest" is retired from the wrapper's vocabulary; it survives only where it means a Cargo manifest.

A collision with the child was **not** the reason. ADR-0044's audit records `--profile` as free in `claude` 2.1.220, so the change rests on coherence; that the retired name was the more likely one for the child to want later is a second-order benefit, not the argument.

`default_profile` is the sole configuration key with no built-in value, so an empty config tree composes nothing and launches the child unchanged. A name that _is_ resolved must exist, at any layer — see [exit codes](../reference/exit-codes.md#resolving-a-profile-name).

## Consequences

- Good: one word to learn, and error messages, paths, and artifacts stop disagreeing.
- Good: the definition no longer has to translate between its own two terms.
- Bad: an XDG layout change, so [ADR-0006](./ADR-0006-place-files-by-xdg-ownership.md)'s artifact table moves with it.
- Bad: "profile" is a common word, so a future child flag could collide where "manifest" would not have.

## Status

Accepted

Amends [ADR-0010](./ADR-0010-compose-native-settings-from-declared-layers.md), whose vocabulary this renames without changing its decision.
