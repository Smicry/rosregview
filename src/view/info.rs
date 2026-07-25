//! `info` subcommand — one-line summary for a hive file.

use crate::cli::OutputFormat;
use crate::hive::open;
use crate::output::Stats;
use anyhow::Result;
use nt_hive::Hive;
use serde::Serialize;
use std::path::Path;

/// Public entry point for the `info` subcommand.
pub fn run(path: &Path, format: OutputFormat) -> Result<()> {
    let (hive, file_size) = open::load_hive(path)?;
    let subkey_count = count_root_subkeys(&hive);

    let stats = Stats::from_hive(path, file_size);
    match format {
        OutputFormat::Human => render_human(&stats, subkey_count),
        OutputFormat::Json => render_json(&stats, subkey_count),
    }
}

fn count_root_subkeys(hive: &Hive<&'static [u8]>) -> usize {
    let root = match hive.root_key_node() {
        Ok(r) => r,
        Err(_) => return 0,
    };
    match root.subkeys() {
        Some(Ok(iter)) => iter.count(),
        _ => 0,
    }
}

#[derive(Debug, Serialize)]
struct InfoPayload {
    #[serde(flatten)]
    base: Stats,
    root_subkey_count: usize,
}

fn render_human(stats: &Stats, subkey_count: usize) -> Result<()> {
    println!("File:           {}", stats.path);
    println!("Size:           {} bytes", stats.file_size_bytes);
    println!("Parsed:         OK (nt-hive 0.3 accepted the file)");
    println!("Root subkeys:   {subkey_count}");
    Ok(())
}

fn render_json(stats: &Stats, subkey_count: usize) -> Result<()> {
    let payload = InfoPayload {
        base: Stats {
            path: stats.path.clone(),
            file_size_bytes: stats.file_size_bytes,
            parsed_ok: stats.parsed_ok,
            minor_version_known: stats.minor_version_known,
        },
        root_subkey_count: subkey_count,
    };
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
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
        // Use the public `load_hive` helper to get a 'static Hive.
        let (hive, _size) = open::load_hive(&p).unwrap();
        // The bundled nt-hive test fixture has exactly 5 root subkeys.
        assert_eq!(count_root_subkeys(&hive), 5);
    }
}
