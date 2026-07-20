//! rosregview — Offline Windows registry hive viewer for ReactOS.
//!
//! See `Cargo.toml` for project metadata, and `rosregview-plan.md` in the
//! repository root for the wider plan and design notes.
//!
//! Subcommands currently implemented:
//!   * `info <hive>`                  — overview + JSON via `-f json`
//!   * `tree <hive> [--depth N]`      — recursive key tree, also JSON
//!
//! Additional subcommands (`list`, `show`, `find`) will land in
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

    /// Recursively print the key tree of a hive file (uses ASCII-only indent).
    Tree {
        /// Path to a .hiv file (Windows registry configuration unit).
        path: PathBuf,

        /// Maximum recursion depth. 0 = show only the root, 1 = root + direct
        /// children, 2 = up to grand-children, ... Default: unlimited.
        #[arg(short = 'd', long = "depth", value_name = "N")]
        depth: Option<usize>,

        /// Output format (`human` is the indented text tree, `json` is machine-readable).
        #[arg(short = 'f', long = "format", value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
}

/// Output formats supported across subcommands.
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

/// Per-hive overview facts (file size, root size, ...).
#[derive(Debug, Serialize)]
struct Stats {
    /// Absolute or user-supplied path the hive was loaded from.
    path: String,
    /// File size in bytes (taken from the OS stat, not the parsed buffer).
    file_size_bytes: u64,
    /// Whether the root key was readable (i.e., nt-hive accepted the buffer).
    parsed_ok: bool,
    /// Placeholder for nt-hive's minor-version field — flag stays `false`
    /// until we add a public getter.
    minor_version_known: bool,
}

/// A single key in the recursive tree (used by both `tree` and `show` paths).
#[derive(Debug, Serialize)]
struct KeyTreeNode {
    name: String,
    subkeys: Vec<KeyTreeNode>,
}

/// Tree-specific stats payload (extends `Stats` with the recursive tree).
#[derive(Debug, Serialize)]
struct TreeStats {
    #[serde(flatten)]
    base: Stats,
    depth_limit: Option<usize>,
    tree: KeyTreeNode,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Info { path, format } => info_command(&path, format),
        Command::Tree { path, depth, format } => tree_command(&path, depth, format),
    }
}

// ----------------------------------------------------------------------
// info
// ----------------------------------------------------------------------

fn info_command(path: &Path, format: OutputFormat) -> Result<()> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read hive file `{}`", path.display()))?;
    let hive = Hive::new(bytes.as_slice()).context("input is not a valid Windows registry hive")?;
    let root = hive
        .root_key_node()
        .context("hive has no root key node")?;

    let file_size = std::fs::metadata(path)
        .context("failed to stat hive file")?
        .len();
    let subkey_count: usize = match root.subkeys() {
        Some(Ok(iter)) => iter.count(),
        Some(Err(e)) => return Err(anyhow::Error::new(e).context("malformed subkey index at root")),
        None => 0,
    };

    let stats = Stats {
        path: path.display().to_string(),
        file_size_bytes: file_size,
        parsed_ok: true,
        minor_version_known: false,
    };

    match format {
        OutputFormat::Human => render_info_human(&stats, subkey_count),
        OutputFormat::Json => render_info_json(&stats, subkey_count),
    }
}

fn render_info_human(stats: &Stats, subkey_count: usize) -> Result<()> {
    println!("File:           {}", stats.path);
    println!("Size:           {} bytes", stats.file_size_bytes);
    println!("Parsed:         OK (nt-hive 0.3 accepted the file)");
    println!("Root subkeys:   {subkey_count}");
    Ok(())
}

fn render_info_json(stats: &Stats, subkey_count: usize) -> Result<()> {
    let value = serde_json::json!({
        "path": stats.path,
        "file_size_bytes": stats.file_size_bytes,
        "parsed_ok": stats.parsed_ok,
        "minor_version_known": stats.minor_version_known,
        "root_subkey_count": subkey_count,
    });
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

// ----------------------------------------------------------------------
// tree
// ----------------------------------------------------------------------

fn tree_command(path: &Path, depth: Option<usize>, format: OutputFormat) -> Result<()> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read hive file `{}`", path.display()))?;
    let hive = Hive::new(bytes.as_slice()).context("input is not a valid Windows registry hive")?;
    let root = hive
        .root_key_node()
        .context("hive has no root key node")?;

    let file_size = std::fs::metadata(path)
        .context("failed to stat hive file")?
        .len();

    // The hive root has no name in the on-disk format; we render it as `<root>`.
    let tree = build_tree(&root, "<root>", depth, 0)?;

    let stats = TreeStats {
        base: Stats {
            path: path.display().to_string(),
            file_size_bytes: file_size,
            parsed_ok: true,
            minor_version_known: false,
        },
        depth_limit: depth,
        tree,
    };

    match format {
        OutputFormat::Human => render_tree_human(&stats),
        OutputFormat::Json => render_tree_json(&stats),
    }
}

/// Recursively walk a `KeyNode` and build a `KeyTreeNode`.
///
/// `depth_limit` semantics: `None` = unlimited; `Some(n)` = stop descending
/// at depth `n` (i.e. n=0 means show only the root, n=1 means root + direct
/// children, ...). `current_depth` is the depth of `node` itself.
fn build_tree<'a>(
    node: &nt_hive::KeyNode<'a, &[u8]>,
    name: &str,
    depth_limit: Option<usize>,
    current_depth: usize,
) -> Result<KeyTreeNode> {
    // Honor the depth limit *before* recursing into children.
    let reached_limit = matches!(depth_limit, Some(limit) if current_depth >= limit);

    let subkeys = if reached_limit {
        Vec::new()
    } else {
        match node.subkeys() {
            Some(Ok(iter)) => iter
                .map(|child_result| -> Result<KeyTreeNode> {
                    let child = child_result.map_err(|e| {
                        anyhow::Error::new(e).context("failed to advance subkey iterator")
                    })?;
                    let child_name = child
                        .name()
                        .map_err(|e| anyhow::Error::new(e).context("failed to read subkey name"))?
                        .to_string_lossy();
                    build_tree(&child, &child_name, depth_limit, current_depth + 1)
                })
                .collect::<Result<Vec<_>>>()?,
            Some(Err(e)) => return Err(anyhow::Error::new(e).context("malformed subkey index")),
            None => Vec::new(),
        }
    };

    Ok(KeyTreeNode {
        name: name.to_string(),
        subkeys,
    })
}

fn render_tree_human(stats: &TreeStats) -> Result<()> {
    println!("File:     {}", stats.base.path);
    println!("Size:     {} bytes", stats.base.file_size_bytes);
    if let Some(limit) = stats.depth_limit {
        println!("Depth:    0..={limit}");
    } else {
        println!("Depth:    unlimited");
    }
    println!();
    print_tree_node(&stats.tree, 0);
    Ok(())
}

fn print_tree_node(node: &KeyTreeNode, depth: usize) {
    println!("{}{}", "  ".repeat(depth), node.name);
    for child in &node.subkeys {
        print_tree_node(child, depth + 1);
    }
}

fn render_tree_json(stats: &TreeStats) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(stats)?);
    Ok(())
}
