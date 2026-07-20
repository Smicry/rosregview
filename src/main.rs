//! rosregview — Offline Windows registry hive viewer for ReactOS.
//!
//! See `Cargo.toml` for project metadata, and `rosregview-plan.md` in the
//! repository root for the wider plan and design notes.
//!
//! This binary is intentionally small for the first cross-compile smoke test:
//! it only implements the `info <hive>` subcommand so we can validate the
//! nt-hive integration end-to-end. Additional subcommands (`tree`, `list`,
//! `show`, `find`) will land in subsequent commits.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use nt_hive::Hive;

/// Offline Windows registry hive viewer for ReactOS.
#[derive(Parser, Debug)]
#[command(name = "rosregview", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Show a one-line summary for a hive file: size, root subkey count.
    Info {
        /// Path to a .hiv file (Windows registry configuration unit).
        path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Info { path } => info_command(&path),
    }
}

fn info_command(path: &PathBuf) -> Result<()> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read hive file `{}`", path.display()))?;

    // nt-hive is zero-copy over the input buffer; it borrows from `bytes`.
    let hive = Hive::new(bytes.as_slice()).context("input is not a valid Windows registry hive")?;
    let root = hive
        .root_key_node()
        .context("hive has no root key node")?;

    let file_size = std::fs::metadata(path)
        .context("failed to stat hive file")?
        .len();

    let subkey_count: usize = match root.subkeys() {
        Some(Ok(iter)) => iter.count(),
        // `Some(Err(_))` would mean the subkey index descriptor itself is
        // malformed. We surface the parse error rather than silently counting 0.
        Some(Err(e)) => return Err(anyhow::Error::new(e).context("malformed subkey index at root")),
        None => 0,
    };

    println!("File:           {}", path.display());
    println!("Size:           {file_size} bytes");
    println!("Parsed:         OK (nt-hive 0.3 accepted the file)");
    println!("Root subkeys:   {subkey_count}");

    Ok(())
}
