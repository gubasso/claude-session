//! The repository's contracts on itself: the decision record shape, the plan
//! zone shape, prose emphasis, the child-owned facts this repository is allowed
//! to carry, and the two architectural boundaries no lint can express.
//!
//! Every gate here reads checked-in repository files and nothing else. It never
//! spawns a process, never uses the network, and never reads host state, so it
//! is a fact about this repository rather than about the machine running it.
//! Reading a file is still the filesystem access the unit lane forbids, which
//! is what puts these in the integration lane and therefore in the push hook
//! rather than the commit hook.
//!
//! The crate root is `main.rs` inside the target directory rather than a
//! sibling `repo_contracts.rs`, because a test target's root file is its own
//! crate root: `mod markdown;` beside such a file would resolve to `tests/`,
//! and every helper would become a stray test binary.
//!
//! Each gate proves it found what it was aiming at before it reports anything.
//! A gate that silently matches zero rows reports success, so every one carries
//! a floor and named sentinels, and every rule is driven from a doctored
//! literal by a negative test. A run that goes green has demonstrated it can go
//! red.

#![allow(
    clippy::expect_used,
    clippy::pedantic,
    clippy::nursery,
    clippy::redundant_pub_crate
)]

mod adrs;
mod boundaries;
mod child_facts;
mod emphasis;
mod markdown;
mod plan_zone;
mod shape;
mod tree;
mod violation;

use std::fs;
use std::path::Path;

/// A missing or unreadable input is a failure naming the path, never a skip. A
/// gate that can quietly decline to run is not a gate.
pub(crate) fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("{relative} is required by the repository gates: {err}"))
}
