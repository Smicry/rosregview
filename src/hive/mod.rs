//! Hive parsing + path navigation.
//!
//! `hive::open` is the "read bytes from disk and feed them to
//! `nt_hive::Hive::new`" entry point shared by every subcommand.
//!
//! `hive::format` provides registry-key-path traversal helpers used
//! by `list`, `show`, and `find` to descend into a subkey.

pub mod format;
pub mod open;
