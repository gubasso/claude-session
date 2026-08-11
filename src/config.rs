//! Strict layered wrapper configuration.

pub(crate) mod load;
pub(crate) mod project;
// `render` is deliberately absent: the binary never renders a development
// artifact, so it is compiled into the library target alone (see src/lib.rs).
pub(crate) mod schema;
