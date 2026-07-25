//! JSON output scaffolding.
//!
//! Currently no helpers are needed: every subcommand builds a typed
//! payload (e.g. `TreeStats`) and prints it via `serde_json::to_string_pretty`.
//! This module exists so future JSON-shape cross-cuts (e.g. a custom
//! value-formatter registry) have an obvious home without re-shuffling
//! every caller.
//!
//! [`crate::output::Stats`] is the canonical base type — `#[serde(flatten)]`
//! on each subcommand's wrapper does the rest.

// Nothing yet — this module is a placeholder by design. Adding a real
// helper here would normally be the result of a third subcommand needing
// the same JSON post-processing.

#[cfg(test)]
mod tests {
    /// Smoke test: serde_json::to_string_pretty on the base `Stats`
    /// includes all four documented fields. Future renames will trip
    /// this test, alerting us to update README/JSON consumers.
    #[test]
    fn stats_base_serializes_all_documented_fields() {
        let stats = super::super::Stats {
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
}
