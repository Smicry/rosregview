//! Text-table helpers shared by the `list` and `show` renderers.
//!
//! Currently this module just re-exports [`crate::hive::format::truncate_with_ellipsis`]
//! under its `output::` path; the indirection lets a future commit move
//! the implementation without touching every caller.

pub use crate::hive::format::truncate_with_ellipsis;
