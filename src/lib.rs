//! The library face of `claude-session`, exporting only what the `xtask`
//! development tooling reflects over.
//!
//! Deliberately narrow. A `lib.rs` that grew into a re-export of every private
//! module would be the dead weight
//! [ADR-0014](../docs/decisions/ADR-0014-xtask-workspace-for-dev-tooling.md)
//! guards against, and the shipped binary would start carrying a public API it
//! never promised.
//!
//! The `#[path]` form compiles the same two files into both targets without
//! pulling the rest of the module tree in. Neither file carries a `///` code
//! fence: adding a library target turns doctests on, and a doc example here
//! would silently become a compiled test.

#[path = "config/schema.rs"]
pub mod schema;

#[path = "config/render.rs"]
pub mod render;
