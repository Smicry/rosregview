//! `info` subcommand — one-line summary for a hive file.

use crate::cli::OutputFormat;
use crate::error;
use crate::hive::open;
use crate::output::{Stats, escape_control_chars, write_stdout};
use anyhow::Result;
use nt_hive::Hive;
use serde::Serialize;
use std::path::Path;

/// Public entry point for the `info` subcommand.
pub fn run(path: &Path, format: OutputFormat) -> Result<()> {
    let (stats, subkey_count) = open::with_hive(path, |hive, file_size| {
        Ok((
            Stats::from_hive(path, file_size, hive.minor_version()),
            count_root_subkeys(hive)?,
        ))
    })?;
    match format {
        OutputFormat::Human => render_human(&stats, subkey_count),
        OutputFormat::Json => render_json(&stats, subkey_count),
    }
}

fn count_root_subkeys(hive: &Hive<&[u8]>) -> Result<usize> {
    let root = hive
        .root_key_node()
        .map_err(|e| error::wrap_hive_error(e, "hive has no readable root key node"))?;
    let count = match root.subkeys() {
        Some(Ok(iter)) => {
            let mut count = 0;
            for child in iter {
                child.map_err(|e| {
                    error::wrap_hive_error(e, "failed to advance root subkey iterator")
                })?;
                count += 1;
            }
            count
        }
        Some(Err(e)) => {
            return Err(error::wrap_hive_error_owned(
                e,
                "malformed subkey index at root".to_string(),
            ));
        }
        None => 0,
    };
    Ok(count)
}

#[derive(Debug, Serialize)]
struct InfoPayload {
    #[serde(flatten)]
    base: Stats,
    root_subkey_count: usize,
}

fn render_human(stats: &Stats, subkey_count: usize) -> Result<()> {
    write_stdout(|out| {
        writeln!(out, "File:           {}", escape_control_chars(&stats.path))?;
        writeln!(out, "Size:           {} bytes", stats.file_size_bytes)?;
        writeln!(
            out,
            "Parsed:         OK (nt-hive 0.3, minor version {})",
            stats.minor_version
        )?;
        writeln!(out, "Root subkeys:   {subkey_count}")?;
        Ok(())
    })
}

fn render_json(stats: &Stats, subkey_count: usize) -> Result<()> {
    let payload = InfoPayload {
        base: Stats {
            path: stats.path.clone(),
            file_size_bytes: stats.file_size_bytes,
            minor_version: stats.minor_version,
        },
        root_subkey_count: subkey_count,
    };
    let json = serde_json::to_string_pretty(&payload)?;
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
    fn count_root_subkeys_matches_known_fixture() {
        let p = fixture_path();
        if !p.is_file() {
            eprintln!("skipping: {} not found", p.display());
            return;
        }
        // Parse through the same scoped helper used by production commands.
        open::with_hive(&p, |hive, _size| {
            assert_eq!(count_root_subkeys(hive).unwrap(), 5);
            Ok(())
        })
        .unwrap();
    }
}
