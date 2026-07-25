//! Cross-subcommand output scaffolding.
//!
//! `Stats` is the JSON-shape contract every subcommand shares under the
//! same top-level keys (`path`, `file_size_bytes`, `minor_version`).
//! Subcommand-specific shapes are layered on top via `#[serde(flatten)]`
//! in each subcommand's `*Stats` wrapper.
//!
//! `output::table`, `output::hex`, and `output::json` provide the small
//! helpers that the per-subcommand render functions reuse.

pub mod hex;
pub mod json;
pub mod table;
pub(crate) mod value;

use serde::Serialize;

/// File-level (hive-wide) facts that every subcommand's JSON output
/// shares under the same top-level keys (`path`, `file_size_bytes`,
/// `minor_version`). Subcommand-specific shape (subkey count for `info`,
/// recursive tree for `tree`, etc.) is layered on top via
/// `#[serde(flatten)]` in each `*Stats` wrapper.
#[derive(Debug, Serialize)]
pub struct Stats {
    /// Absolute or user-supplied path the hive was loaded from.
    pub path: String,
    /// File size in bytes (taken from the OS stat, not the parsed buffer).
    pub file_size_bytes: u64,
    /// The hive's minor version (e.g. 5 = Windows XP), read directly
    /// from the base block via `nt_hive::Hive::minor_version`.
    /// Consumers can map this to a known Windows version using
    /// `nt_hive::HiveMinorVersion::n`.
    pub minor_version: u32,
}

impl Stats {
    /// Construct the `Stats` shared base from a hive path, OS file
    /// size, and the hive's minor version. Centralised here so all
    /// subcommands agree on the field order / defaults.
    pub fn from_hive(path: &std::path::Path, file_size_bytes: u64, minor_version: u32) -> Self {
        Self {
            path: path.display().to_string(),
            file_size_bytes,
            minor_version,
        }
    }
}
