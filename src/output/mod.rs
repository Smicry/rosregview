//! Cross-subcommand output scaffolding.
//!
//! `Stats` is the JSON-shape contract every subcommand shares under the
//! same top-level keys (`path`, `file_size_bytes`, `parsed_ok`,
//! `minor_version_known`). Subcommand-specific shapes are layered on
//! top via `#[serde(flatten)]` in each subcommand's `*Stats` wrapper.
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
/// `parsed_ok`, `minor_version_known`). Subcommand-specific shape
/// (subkey count for `info`, recursive tree for `tree`, etc.) is layered
/// on top via `#[serde(flatten)]` in each `*Stats` wrapper.
#[derive(Debug, Serialize)]
pub struct Stats {
    /// Absolute or user-supplied path the hive was loaded from.
    pub path: String,
    /// File size in bytes (taken from the OS stat, not the parsed buffer).
    pub file_size_bytes: u64,
    /// Whether the root key was readable (i.e., nt-hive accepted the buffer).
    pub parsed_ok: bool,
    /// Placeholder for nt-hive's minor-version field — flag stays
    /// `false` until we add a public getter.
    pub minor_version_known: bool,
}

impl Stats {
    /// Construct the `Stats` shared base from a hive path + OS file
    /// size. Centralised here so all subcommands agree on the field
    /// order / defaults.
    pub fn from_hive(path: &std::path::Path, file_size_bytes: u64) -> Self {
        Self {
            path: path.display().to_string(),
            file_size_bytes,
            parsed_ok: true,
            minor_version_known: false,
        }
    }
}
