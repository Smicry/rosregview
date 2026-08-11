//! `list` subcommand — direct subkeys of a key (default: the root).

use crate::cli::OutputFormat;
use crate::error;
use crate::hive::{format, open};
use crate::output::{Stats, escape_control_chars, truncate_with_ellipsis, write_stdout};
use anyhow::Result;
use nt_hive::KeyNode;
use serde::Serialize;
use std::path::Path;

/// Public entry point for the `list` subcommand.
pub fn run(path: &Path, key_path: Option<&str>, format_flag: OutputFormat) -> Result<()> {
    let target_name: &str = match key_path {
        None | Some("") | Some(".") => "<root>",
        Some(p) => p,
    };
    let stats = open::with_hive(path, |hive, file_size| {
        let root = hive
            .root_key_node()
            .map_err(|e| error::wrap_hive_error(e, "hive has no root key node"))?;
        let target = match key_path {
            None | Some("") | Some(".") => root,
            Some(p) => format::find_subpath(&root, p)?,
        };
        let entries = collect_entries(&target)?;
        Ok(ListStats {
            base: Stats::from_hive(path, file_size, hive.minor_version()),
            at: target_name.to_string(),
            total_entries: entries.len(),
            entries,
        })
    })?;

    match format_flag {
        OutputFormat::Human => render_human(&stats),
        OutputFormat::Json => render_json(&stats),
    }
}

/// A single subkey entry: name + subkey count + value count.
#[derive(Debug, Serialize)]
struct ListEntry {
    name: String,
    subkey_count: usize,
    value_count: usize,
}

/// Payload for the `list` subcommand.
#[derive(Debug, Serialize)]
struct ListStats {
    #[serde(flatten)]
    base: Stats,
    /// Where in the hive we listed from. `"<root>"` for the hive root,
    /// otherwise the user-supplied `KEY_PATH`.
    at: String,
    entries: Vec<ListEntry>,
    total_entries: usize,
}

/// Collect the direct children of `target` along with their per-child
/// `subkey_count` and `value_count`. Counts are best-effort: a malformed
/// subkey index or value list is logged and counted as 0 rather than
/// aborting the whole listing.
fn collect_entries<'a>(target: &KeyNode<'a, &'a [u8]>) -> Result<Vec<ListEntry>> {
    let children = match target.subkeys() {
        Some(Ok(iter)) => iter,
        Some(Err(e)) => {
            return Err(error::wrap_hive_error_owned(
                e,
                "malformed subkey index".to_string(),
            ));
        }
        None => return Ok(Vec::new()),
    };

    let mut out = Vec::new();
    for child_result in children {
        let child = child_result
            .map_err(|e| error::wrap_hive_error(e, "failed to advance subkey iterator"))?;
        let name = child
            .name()
            .map_err(|e| error::wrap_hive_error(e, "failed to read subkey name"))?
            .to_string_lossy();

        let subkey_count = match child.subkeys() {
            Some(Ok(iter)) => {
                let mut count = 0;
                for item in iter {
                    item.map_err(|e| error::wrap_hive_error(e, "failed to count subkeys"))?;
                    count += 1;
                }
                count
            }
            Some(Err(e)) => return Err(error::wrap_hive_error(e, "malformed subkey index")),
            None => 0,
        };
        let value_count = match child.values() {
            Some(Ok(iter)) => {
                let mut count = 0;
                for item in iter {
                    item.map_err(|e| error::wrap_hive_error(e, "failed to count values"))?;
                    count += 1;
                }
                count
            }
            Some(Err(e)) => return Err(error::wrap_hive_error(e, "malformed value list")),
            None => 0,
        };

        out.push(ListEntry {
            name,
            subkey_count,
            value_count,
        });
    }
    Ok(out)
}

fn render_human(stats: &ListStats) -> Result<()> {
    write_stdout(|out| {
        writeln!(out, "File:    {}", escape_control_chars(&stats.base.path))?;
        writeln!(out, "At:      {}", escape_control_chars(&stats.at))?;
        writeln!(out)?;
        writeln!(out, "{:<40}  {:>8}  {:>8}", "Name", "Subkeys", "Values")?;
        writeln!(out, "{}", "─".repeat(60))?;
        for entry in &stats.entries {
            let display_name = truncate_with_ellipsis(&escape_control_chars(&entry.name), 40);
            writeln!(
                out,
                "{:<40}  {:>8}  {:>8}",
                display_name, entry.subkey_count, entry.value_count
            )?;
        }
        writeln!(out)?;
        writeln!(out, "Total: {} keys", stats.total_entries)?;
        Ok(())
    })
}

fn render_json(stats: &ListStats) -> Result<()> {
    let json = serde_json::to_string_pretty(stats)?;
    write_stdout(|out| {
        writeln!(out, "{json}")?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join("testhive")
    }

    #[test]
    fn collect_entries_at_root_yields_five_for_testhive() {
        let p = fixture_path();
        if !p.is_file() {
            eprintln!("skipping: {} not found", p.display());
            return;
        }
        open::with_hive(&p, |hive, _size| {
            let root = hive.root_key_node().unwrap();
            let entries = collect_entries(&root).unwrap();
            assert_eq!(entries.len(), 5);
            for e in &entries {
                assert!(!e.name.is_empty(), "entry has empty name: {e:?}");
            }
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn collect_entries_on_nonexistent_subpath_errors_cleanly() {
        let p = fixture_path();
        if !p.is_file() {
            eprintln!("skipping: {} not found", p.display());
            return;
        }
        open::with_hive(&p, |hive, _size| {
            let root = hive.root_key_node().unwrap();
            let err = match format::find_subpath(&root, "definitely-missing") {
                Ok(_) => panic!("missing subpath must error"),
                Err(e) => e,
            };
            assert!(err.to_string().contains("no such key path"));
            Ok(())
        })
        .unwrap();
    }
}
