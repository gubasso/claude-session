# ADR-0087: Keep the credential lock beside the account it guards

## Context and Problem Statement

[ADR-0069](./ADR-0069-destroy-the-credential-lock-with-its-scope.md) put `.credentials.lock` inside the account directory and had removal delete it along with the tree. Its claimed benefit — that `remove` and `login` cannot interleave — does not hold. Removal must unlink the sentinel before it can `rmdir` the directory, and acquisition creates the sentinel when absent, so a login arriving between those two syscalls creates a second inode, locks it, and excludes nobody. Both processes then believe they hold the scope.

## Considered Options

- Detect the collision afterwards, from the `ENOTEMPTY` the removal's `rmdir` returns.
- Rename the account directory aside, then destroy the detached tree.
- Move the lock beside the account, at `accounts/.<account>.lock`, and never delete it.

## Decision Outcome

Chosen option: move the lock beside the account. A lock whose identity a racer can recreate is not a lock, and the identity is recreatable precisely because the sentinel lives inside the thing being destroyed. Outside it, the inode is stable across the whole removal and ADR-0060's never-deleted rule applies unamended.

ADR-0069 rejected this to keep one artifact-table rule, and to avoid a phantom account left behind. Neither cost is real for a sibling file: it is beside the account exactly as the rule intends, and [account discovery](../reference/accounts.md#what-an-account-is) enumerates directories whose names parse as identifiers, which a leading dot fails twice over. What ADR-0069 actually avoided was a leftover empty directory, which this is not.

Detecting the collision afterwards reports a race rather than preventing one. Renaming aside makes destruction atomic but adds a transient artifact, a table row, and a sweep rule to buy what a stable inode gives for free.

## Consequences

- Good: acquisition and removal exclude each other for the whole removal.
- Good: the lock file is never deleted, so [ADR-0060](./ADR-0060-lock-the-writes-that-are-not-derivable.md) needs no exception.
- Bad: one empty lock file per account outlives it, invisible to every report.

## Status

Implemented

Supersedes [ADR-0069](./ADR-0069-destroy-the-credential-lock-with-its-scope.md). The scope and removal order are in [XDG storage](../reference/xdg-storage.md#lock-scopes).
