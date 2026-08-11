//! `find` subcommand — pattern search across the key tree.

use crate::cli::OutputFormat;
use crate::error;
use crate::hive::open;
use crate::output::{
    Stats, escape_control_chars,
    value::{format_value_data, reg_type_label},
    write_stdout,
};
use anyhow::Result;
use nt_hive::KeyNode;
use serde::Serialize;
use std::path::Path;

const MAX_TRAVERSAL_DEPTH: usize = 512;
const MAX_SCANNED_KEYS: usize = 1_000_000;

/// Public entry point for the `find` subcommand.
#[allow(clippy::too_many_arguments)]
pub fn run(
    path: &Path,
    name: &[String],
    value: Option<&str>,
    case_sensitive: bool,
    max_depth: Option<usize>,
    format: OutputFormat,
) -> Result<()> {
    let patterns = FindPatterns {
        name: name.to_vec(),
        value: value.map(|s| s.to_string()),
        case_sensitive,
    };
    let stats = open::with_hive(path, |hive, file_size| {
        let root = hive
            .root_key_node()
            .map_err(|e| error::wrap_hive_error(e, "hive has no root key node"))?;
        let mut matches = Vec::new();
        let mut total_keys = 0;
        walk_for_matches(
            &root,
            "<root>",
            0,
            max_depth,
            &patterns,
            &mut matches,
            &mut total_keys,
        )?;
        Ok(FindStats {
            base: Stats::from_hive(path, file_size, hive.minor_version()),
            patterns,
            max_depth,
            matches,
            total_keys,
        })
    })?;

    match format {
        OutputFormat::Human => render_human(&stats),
        OutputFormat::Json => render_json(&stats),
    }
}

/// Find-command filter state.
///
/// Compound OR semantics across both lists:
///   * `name` is a list of substring patterns (any-of match): if any
///     pattern passes, the key's name filter passes.
///   * `value` is at most a single substring pattern that is matched
///     against the value's name AND its stringified data preview
///     (the same text `show` renders).
///
/// A key is collected iff its **name filter** passes OR at least one of
/// its **values** passes the value filter. With both lists empty
/// (default), every key passes.
///
/// Why this shape instead of regex:
///   * Plain substring avoids pulling in the `regex` crate just so
///     users can write `displayname=*driver*`.
///   * Single global `case_sensitive` flag is enough for the 95% case.
///   * Per-pattern flags would be ergonomic sugar but add API surface
///     without buying much for a CLI.
///
/// Why match values against the *stringified* preview, not raw bytes:
///   * `show`'s `format_value_data` already does the type-aware decode.
///     Reusing it lets `find -v 42` find both the decimal and the hex
///     representation of a REG_DWORD.
///   * Searching REG_BINARY bytes directly is not supported — users
///     who need that can pipe `find -f json` into `jq` to filter on
///     the `matched_values[*].preview` field.
#[derive(Debug, Clone, Default, Serialize)]
struct FindPatterns {
    name: Vec<String>,
    value: Option<String>,
    case_sensitive: bool,
}

impl FindPatterns {
    fn matches_name(&self, haystack: &str) -> bool {
        if self.name.is_empty() {
            // With no filters, find enumerates every key. A value-only
            // search must not let the absent name filter match every key.
            return self.value.is_none();
        }
        self.name
            .iter()
            .any(|p| fuzzy_contains(haystack, p, self.case_sensitive))
    }

    fn matches_value(&self, haystack: &str) -> bool {
        if let Some(p) = &self.value {
            fuzzy_contains(haystack, p, self.case_sensitive)
        } else {
            false
        }
    }
}

/// True iff `needle` occurs in `haystack`, with case folded by default.
fn fuzzy_contains(haystack: &str, needle: &str, case_sensitive: bool) -> bool {
    if needle.is_empty() {
        return true;
    }
    if case_sensitive {
        haystack.contains(needle)
    } else {
        // Equivalence classes agree on ASCII; Unicode folding is
        // approximate but matches common registry-name patterns in
        // our test data.
        haystack.to_lowercase().contains(&needle.to_lowercase())
    }
}

/// A single match — a key whose name matched (or whose value matched
/// if the user asked for value matching). `matched_values` is non-empty
/// when a value pattern matched and was the reason the key was
/// collected.
#[derive(Debug, Serialize)]
struct KeyMatch {
    key_path: String,
    depth: usize,
    matched_values: Vec<ValueMatchHint>,
}

/// Hint record per value that participated in a match. We keep just
/// enough fields for both the JSON and human sinks without re-walking
/// the hive.
#[derive(Debug, Serialize)]
struct ValueMatchHint {
    name: String,
    reg_type: String,
    preview: String,
}

/// Payload for the `find` subcommand.
#[derive(Debug, Serialize)]
struct FindStats {
    #[serde(flatten)]
    base: Stats,
    patterns: FindPatterns,
    max_depth: Option<usize>,
    matches: Vec<KeyMatch>,
    total_keys: usize,
}

/// Recursively walk the hive, descending only while `depth_limit`
/// permits. Records a match whenever the key's name passes the name
/// filter OR at least one of its values passes the value filter.
fn walk_for_matches<'a>(
    node: &KeyNode<'a, &'a [u8]>,
    path_label: &str,
    depth: usize,
    depth_limit: Option<usize>,
    patterns: &FindPatterns,
    out: &mut Vec<KeyMatch>,
    total_keys: &mut usize,
) -> Result<()> {
    if depth > MAX_TRAVERSAL_DEPTH {
        anyhow::bail!(
            "registry key nesting exceeds the safety limit of {MAX_TRAVERSAL_DEPTH} levels"
        );
    }
    if *total_keys >= MAX_SCANNED_KEYS {
        anyhow::bail!("registry contains more than {MAX_SCANNED_KEYS} keys");
    }
    *total_keys += 1;

    let name = node
        .name()
        .map_err(|e| error::wrap_hive_error(e, "failed to read key name during find"))?
        .to_string_lossy();
    let display_name: &str = if path_label == "<root>" && depth == 0 {
        "<root>"
    } else {
        &name
    };

    // Compute value matches first so we always collect them when they
    // exist, regardless of whether the name filter matched.
    let mut matched_values = Vec::new();
    if patterns.value.is_some() {
        match node.values() {
            Some(Ok(iter)) => {
                for val_result in iter {
                    let val = val_result.map_err(|e| {
                        error::wrap_hive_error(e, "failed to advance value iterator during find")
                    })?;
                    let val_name = val
                        .name()
                        .map_err(|e| error::wrap_hive_error(e, "failed to read value name"))?
                        .to_string_lossy();
                    let reg_type = match val.data_type() {
                        Ok(t) => reg_type_label(t).to_string(),
                        Err(_) => "REG_UNKNOWN".to_string(),
                    };
                    let formatted = match format_value_data(&val, &reg_type, false) {
                        Ok(data) => data,
                        Err(e) => {
                            eprintln!(
                                "rosregview: warning: failed to decode value `{val_name}` at `{path_label}`: {e}"
                            );
                            continue;
                        }
                    };

                    if patterns.matches_value(&val_name)
                        || patterns.matches_value(&formatted.search)
                    {
                        matched_values.push(ValueMatchHint {
                            name: val_name,
                            reg_type,
                            preview: formatted.human,
                        });
                    }
                }
            }
            Some(Err(e)) => return Err(error::wrap_hive_error(e, "malformed value list")),
            None => {} // No value list on this key — normal for leaf keys.
        }
    }

    let name_passes = patterns.matches_name(display_name);
    let value_passes = !matched_values.is_empty();

    // A key is collected iff at least one of the two filters matched
    // it. (If neither filter is provided, `name_passes` is true and
    // `value_passes` is false → everything is collected.)
    if name_passes || value_passes {
        out.push(KeyMatch {
            key_path: path_label.to_string(),
            depth,
            matched_values,
        });
    }

    // Descend only while we're within the depth budget. depth_limit=0
    // means: show only root, do not descend.
    if matches!(depth_limit, Some(limit) if depth >= limit) {
        return Ok(());
    }

    match node.subkeys() {
        Some(Ok(iter)) => {
            for child_result in iter {
                let child = child_result.map_err(|e| {
                    error::wrap_hive_error(e, "failed to advance subkey iterator during find")
                })?;
                let child_name = child
                    .name()
                    .map_err(|e| error::wrap_hive_error(e, "failed to read subkey name"))?
                    .to_string_lossy();
                let child_path = if path_label == "<root>" {
                    child_name.clone()
                } else {
                    format!("{path_label}\\{child_name}")
                };
                walk_for_matches(
                    &child,
                    &child_path,
                    depth + 1,
                    depth_limit,
                    patterns,
                    out,
                    total_keys,
                )?;
            }
        }
        Some(Err(e)) => return Err(error::wrap_hive_error(e, "malformed subkey index")),
        None => {}
    }

    Ok(())
}

fn render_human(stats: &FindStats) -> Result<()> {
    write_stdout(|out| {
        writeln!(out, "File:     {}", escape_control_chars(&stats.base.path))?;
        writeln!(
            out,
            "Patterns: name~={:?}  value~={:?}  case_sensitive={}",
            stats.patterns.name, stats.patterns.value, stats.patterns.case_sensitive
        )?;
        if let Some(limit) = stats.max_depth {
            writeln!(out, "Max depth: {limit}")?;
        } else {
            writeln!(out, "Max depth: unlimited")?;
        }
        writeln!(
            out,
            "Scanned {} keys, matched {} key(s).",
            stats.total_keys,
            stats.matches.len()
        )?;
        if !stats.matches.is_empty() {
            writeln!(out)?;
        }
        for m in &stats.matches {
            let indent = "  ".repeat(m.depth);
            writeln!(out, "{indent}{}", escape_control_chars(&m.key_path))?;
            for v in &m.matched_values {
                writeln!(
                    out,
                    "{}    • {}: {} = {}",
                    indent,
                    escape_control_chars(&v.name),
                    v.reg_type,
                    escape_control_chars(&v.preview)
                )?;
            }
        }
        Ok(())
    })
}

fn render_json(stats: &FindStats) -> Result<()> {
    let json = serde_json::to_string_pretty(stats)?;
    write_stdout(|out| {
        writeln!(out, "{json}")?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_contains_empty_needle_matches_anything() {
        assert!(fuzzy_contains("anything", "", false));
        assert!(fuzzy_contains("", "", false));
        assert!(fuzzy_contains("anything", "", true));
    }

    #[test]
    fn fuzzy_contains_is_case_insensitive_by_default() {
        assert!(fuzzy_contains("Hello World", "hello", false));
        assert!(fuzzy_contains("Hello World", "WORLD", false));
        assert!(!fuzzy_contains("Hello World", "hello", true));
        assert!(fuzzy_contains("Hello World", "Hello", true));
    }

    #[test]
    fn fuzzy_contains_negative() {
        assert!(!fuzzy_contains("hello", "world", true));
        assert!(!fuzzy_contains("hello", "world", false));
    }

    #[test]
    fn find_patterns_without_filters_accepts_every_name() {
        let p = FindPatterns::default();
        assert!(p.matches_name("anything"));
        assert!(p.matches_name(""));
    }

    #[test]
    fn find_patterns_value_only_filter_does_not_match_names() {
        let p = FindPatterns {
            value: Some("42".to_string()),
            ..Default::default()
        };
        assert!(!p.matches_name("anything"));
    }

    #[test]
    fn find_patterns_name_filter_any_of() {
        let p = FindPatterns {
            name: vec!["foo".to_string(), "bar".to_string()],
            ..Default::default()
        };
        assert!(p.matches_name("foo_baz"));
        assert!(p.matches_name("qux_bar"));
        assert!(!p.matches_name("qux_baz"));
    }

    #[test]
    fn find_patterns_value_filter_requires_pattern() {
        let p = FindPatterns::default();
        // No value pattern → never matches.
        assert!(!p.matches_value("anything"));

        let p = FindPatterns {
            value: Some("42".to_string()),
            ..Default::default()
        };
        assert!(p.matches_value("the answer is 42"));
    }

    #[test]
    fn find_patterns_value_filter_honors_case_sensitive() {
        let p = FindPatterns {
            value: Some("Foo".to_string()),
            case_sensitive: true,
            ..Default::default()
        };
        assert!(p.matches_value("hello Foo"));
        assert!(!p.matches_value("hello foo"));
    }
}
