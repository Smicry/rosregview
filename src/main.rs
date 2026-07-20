//! rosregview — Offline Windows registry hive viewer for ReactOS.
//!
//! See `Cargo.toml` for project metadata, and `rosregview-plan.md` in the
//! repository root for the wider plan and design notes.
//!
//! This binary currently implements the `info <hive>` subcommand, both as
//! a human-readable summary and as a structured JSON payload (via `-f json`).
//! Additional subcommands (`tree`, `list`, `show`, `find`) will land in
//! subsequent commits and reuse the same `Stats` data flow.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use nt_hive::Hive;
use serde::Serialize;

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

        /// Output format (`human` is the default table, `json` is machine-readable).
        #[arg(short = 'f', long = "format", value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
}

/// Output formats supported across subcommands.
///
/// New variants added here will become available everywhere `Stats` is
/// rendered (currently: `info`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

impl FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "human" => Ok(Self::Human),
            "json" => Ok(Self::Json),
            other => Err(format!("unknown output format `{other}` (expected: human, json)")),
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Human => f.write_str("human"),
            OutputFormat::Json => f.write_str("json"),
        }
    }
}

/// Structured facts about a parsed hive. This is the single source of data
/// that all output formats render — so adding a new format does not require
/// re-parsing the hive.
#[derive(Debug, Serialize)]
struct Stats {
    /// Absolute or user-supplied path the hive was loaded from.
    path: String,
    /// File size in bytes (taken from the OS stat, not the parsed buffer).
    file_size_bytes: u64,
    /// Number of direct subkeys under the root KeyNode.
    root_subkey_count: usize,
    /// nt-hive's minor-version field, when known (raw u32 for now).
    minor_version_known: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Info { path, format } => info_command(&path, format),
    }
}

fn info_command(path: &Path, format: OutputFormat) -> Result<()> {
    let stats = compute_stats(path)?;
    match format {
        OutputFormat::Human => render_human(&stats),
        OutputFormat::Json => render_json(&stats),
    }
}

fn compute_stats(path: &Path) -> Result<Stats> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read hive file `{}`", path.display()))?;

    let hive = Hive::new(bytes.as_slice()).context("input is not a valid Windows registry hive")?;
    let root = hive
        .root_key_node()
        .context("hive has no root key node")?;

    let file_size = std::fs::metadata(path)
        .context("failed to stat hive file")?
        .len();

    let root_subkey_count: usize = match root.subkeys() {
        Some(Ok(iter)) => iter.count(),
        // `Some(Err(_))` would mean the subkey index descriptor itself is
        // malformed. Surface the parse error rather than silently counting 0.
        Some(Err(e)) => return Err(anyhow::Error::new(e).context("malformed subkey index at root")),
        None => 0,
    };

    Ok(Stats {
        path: path.display().to_string(),
        file_size_bytes: file_size,
        root_subkey_count,
        // We don't currently extract the minor-version field through nt-hive
        // 0.3's public API; this flag stays `false` until we add a getter,
        // at which point JSON consumers can safely switch on it.
        minor_version_known: false,
    })
}

fn render_human(stats: &Stats) -> Result<()> {
    println!("File:           {}", stats.path);
    println!("Size:           {} bytes", stats.file_size_bytes);
    println!("Parsed:         OK (nt-hive 0.3 accepted the file)");
    println!("Root subkeys:   {}", stats.root_subkey_count);
    Ok(())
}

fn render_json(stats: &Stats) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(stats)?);
    Ok(())
}
