//! `info` subcommand — one-line summary for a hive file.

use crate::cli::OutputFormat;
use crate::error;
use crate::hive::open;
use crate::output::Stats;
use anyhow::Result;
use nt_hive::Hive;
use serde::Serialize;
use std::path::Path;

/// Public entry point for the `info` subcommand.
pub fn run(path: &Path, format: OutputFormat) -> Result<()> {
    let (hive, file_size) = open::load_hive(path)?;
    let subkey_count = count_root_subkeys(&hive)?;

    let stats = Stats::from_hive(path, file_size, hive.minor_version());
    match format {
        OutputFormat::Human => render_human(&stats, subkey_count),
        OutputFormat::Json => render_json(&stats, subkey_count),
    }
}

fn count_root_subkeys(hive: &Hive<&'static [u8]>) -> Result<usize> {
    let root = hive
        .root_key_node()
        .map_err(|e| error::wrap_hive_error(e, "hive has no readable root key node"))?;
    let count = match root.subkeys() {
        Some(Ok(iter)) => iter.count(),
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
    println!("File:           {}", stats.path);
    println!("Size:           {} bytes", stats.file_size_bytes);
    println!(
        "Parsed:         OK (nt-hive 0.3, minor version {})",
        stats.minor_version
    );
    println!("Root subkeys:   {subkey_count}");
    Ok(())
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
        assert_eq!(count_root_subkeys(&hive).unwrap(), 5);
    }
}
