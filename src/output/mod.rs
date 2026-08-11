//! Cross-subcommand output scaffolding.
//!
//! `Stats` is the JSON-shape contract every subcommand shares under the
//! same top-level keys (`path`, `file_size_bytes`, `minor_version`).
//! Subcommand-specific shapes are layered on top via `#[serde(flatten)]`
//! in each subcommand's `*Stats` wrapper.
//!
//! `truncate_with_ellipsis` lives here as the shared output-formatting
//! helper; `output::hex` and `output::value` provide the hex-dump and
//! typed value decoders respectively.
//!
//! (The former `output::json` and `output::table` sub-modules were
//! removed: the former was a comments-only placeholder, the latter a
//! single re-export line. Their responsibilities — if any real code
//! ever appears — belong here in the module root.)

pub mod hex;
pub(crate) mod value;

use serde::Serialize;
use std::io::{BufWriter, Write};

/// Write a complete response through a locked, buffered stdout and propagate
/// I/O failures so the binary can handle broken pipes without panicking.
pub fn write_stdout(f: impl FnOnce(&mut dyn Write) -> anyhow::Result<()>) -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    f(&mut writer)?;
    writer.flush()?;
    Ok(())
}

/// Escape terminal control characters while preserving printable Unicode.
pub fn escape_control_chars(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        if c.is_control() {
            escaped.extend(c.escape_default());
        } else {
            escaped.push(c);
        }
    }
    escaped
}

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
    /// `nt_hive::HiveMinorVersion::n`; see the README for a value map.
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

/// Truncate a string to `max_chars` characters, appending `…` if it was
/// longer. Counts by Unicode scalar value (Rust `char`), not bytes.
///
/// Examples (where `max_chars = 5`):
///   * `"hello"`     → `"hello"`
///   * `"hello!!"`   → `"hell…"`
///   * `"héllo"`     → `"héllo"`  (5 chars, even though 6 bytes)
pub fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else {
        let keep = max_chars.saturating_sub(1);
        let truncated: String = s.chars().take(keep).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: serde_json::to_string_pretty on the base `Stats`
    /// includes all documented fields. Future renames will trip
    /// this test, alerting us to update README/JSON consumers.
    #[test]
    fn stats_base_serializes_all_documented_fields() {
        let stats = Stats {
            path: "x.hiv".into(),
            file_size_bytes: 42,
            minor_version: 5,
        };
        let json: serde_json::Value = serde_json::to_value(&stats).unwrap();
        let obj = json.as_object().unwrap();
        for required in ["path", "file_size_bytes", "minor_version"] {
            assert!(obj.contains_key(required), "missing `{required}`");
        }
        assert_eq!(obj["file_size_bytes"].as_u64(), Some(42));
        assert_eq!(obj["minor_version"].as_u64(), Some(5));
    }

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
        assert_eq!(truncate_with_ellipsis("hi", 5), "hi");
    }

    #[test]
    fn truncate_long_string_with_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello world", 5), "hell…");
    }

    #[test]
    fn truncate_empty_string_is_empty() {
        assert_eq!(truncate_with_ellipsis("", 5), "");
    }

    #[test]
    fn truncate_counts_chars_not_bytes() {
        // `héllo` is 5 chars (6 bytes). max_chars=5 should NOT truncate.
        assert_eq!(truncate_with_ellipsis("héllo", 5), "héllo");
        // 6 chars: h, é, l, l, o, !. Keep first 4 + ellipsis → "héll…".
        assert_eq!(truncate_with_ellipsis("héllo!", 5), "héll…");
        // Pure ASCII sanity.
        assert_eq!(truncate_with_ellipsis("abcdef", 5), "abcd…");
    }

    #[test]
    fn truncate_max_chars_zero_yields_only_ellipsis() {
        // saturating_sub gives 0; we still emit the ellipsis sentinel.
        assert_eq!(truncate_with_ellipsis("anything", 0), "…");
    }

    #[test]
    fn escape_control_chars_preserves_text_and_escapes_controls() {
        assert_eq!(
            escape_control_chars("ok\n\u{1b}[31m雪"),
            "ok\\n\\u{1b}[31m雪"
        );
    }
}
