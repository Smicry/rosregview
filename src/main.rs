//! rosregview — Offline Windows registry hive viewer for ReactOS.
//!
//! See `Cargo.toml` for project metadata, and `rosregview-plan.md` in the
//! repository root for the wider plan and design notes.
//!
//! Subcommands currently implemented:
//!   * `info <hive>`                  — overview + JSON via `-f json`
//!   * `tree <hive> [--depth N]`      — recursive key tree, also JSON
//!   * `list <hive> [PATH]`           — direct children at PATH, also JSON
//!   * `show <hive> [PATH]`           — values of a key, also JSON
//!   * `find <hive> [filters]`        — pattern search across the key tree

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

    /// List the direct subkeys of a key (default: the root), with subkey and
    /// value counts. The optional `KEY_PATH` may be a single segment
    /// (`ControlSet001`) or a backslash-separated subpath (`A\B\C`). Empty
    /// path == the root.
    List {
        /// Path to a .hiv file (Windows registry configuration unit).
        path: PathBuf,

        /// Optional key path inside the hive. Use `\` (escaped as `\\` in
        /// most shells) to separate levels. Empty/missing → hive root.
        #[arg(value_name = "KEY_PATH")]
        key_path: Option<String>,

        /// Output format (`human` is an aligned table, `json` is machine-readable).
        #[arg(short = 'f', long = "format", value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },

    /// Show the values of a key (default: the root). Per-value output
    /// includes the value name, its REG_* type, and the data, decoded
    /// according to the type where possible (UTF-16 strings, u32 little-
    /// endian dwords, u64 little-endian qwords, ...). Binary data is shown
    /// as a hex dump; long strings are truncated with a trailing `…`.
    Show {
        /// Path to a .hiv file (Windows registry configuration unit).
        path: PathBuf,

        /// Optional key path inside the hive (same convention as `list`).
        #[arg(value_name = "KEY_PATH")]
        key_path: Option<String>,

        /// Output format (`human` is a typed table, `json` is machine-readable).
        #[arg(short = 'f', long = "format", value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },

    /// Search the key hierarchy for keys with names matching `-n` (substring,
    /// repeatable, any-of, case-insensitive by default) and/or values whose
    /// name+decoded-data contains `-v`. With no filters, this enumerates
    /// every key path up to `--max-depth`.
    Find {
        /// Path to a .hiv file (Windows registry configuration unit).
        path: PathBuf,

        /// Substring to match in key names. Repeat to OR multiple patterns.
        #[arg(short = 'n', long = "name", value_name = "PATTERN")]
        name: Vec<String>,

        /// Substring to match in value name + decoded data (any value within
        /// a matching key whose name/data contains the pattern).
        #[arg(short = 'v', long = "value", value_name = "PATTERN")]
        value: Option<String>,

        /// Match case-sensitively. Default: case-insensitive.
        #[arg(long = "case-sensitive", default_value_t = false)]
        case_sensitive: bool,

        /// Limit recursion depth. 0 = root only, 1 = root + direct
        /// children, ... Default: unlimited.
        #[arg(long = "max-depth", value_name = "N")]
        max_depth: Option<usize>,

        /// Output format (`human` is a path list, `json` is machine-readable).
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

/// A single key in the recursive tree. Used only by `tree`; `show` has
/// its own `ValueEntry`/`ListEntry` types because values and subkeys are
/// decoded along different paths (typed `KeyValueData` vs raw key index).
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
        Command::List { path, key_path, format } => list_command(&path, key_path.as_deref(), format),
        Command::Show { path, key_path, format } => show_command(&path, key_path.as_deref(), format),
        Command::Find {
            path,
            name,
            value,
            case_sensitive,
            max_depth,
            format,
        } => find_command(
            &path,
            &name,
            value.as_deref(),
            case_sensitive,
            max_depth,
            format,
        ),
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
    node: &nt_hive::KeyNode<'a, &'a [u8]>,
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

// ----------------------------------------------------------------------
// list
// ----------------------------------------------------------------------

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

fn list_command(path: &Path, key_path: Option<&str>, format: OutputFormat) -> Result<()> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read hive file `{}`", path.display()))?;
    let hive = Hive::new(bytes.as_slice()).context("input is not a valid Windows registry hive")?;
    let root = hive
        .root_key_node()
        .context("hive has no root key node")?;

    // Resolve the target KeyNode from `key_path`. None / empty / "."  → root.
    let target = match key_path {
        None | Some("") | Some(".") => root,
        Some(p) => find_subpath(&root, p)?,
    };
    let target_name: &str = match key_path {
        None | Some("") | Some(".") => "<root>",
        Some(p) => p,
    };

    let entries = list_entries(&target)?;

    let stats = ListStats {
        base: Stats {
            path: path.display().to_string(),
            file_size_bytes: std::fs::metadata(path)
                .context("failed to stat hive file")?
                .len(),
            parsed_ok: true,
            minor_version_known: false,
        },
        at: target_name.to_string(),
        total_entries: entries.len(),
        entries,
    };

    match format {
        OutputFormat::Human => render_list_human(&stats),
        OutputFormat::Json => render_list_json(&stats),
    }
}

/// Locate a subkey under `root` given a backslash-separated path. Returns
/// the `KeyNode` at the end of the path, or an error if no such path exists
/// in the hive.
///
/// We rely on `nt_hive::KeyNode::subpath` which performs the descent
/// internally — no manual segment traversal on our side.
fn find_subpath<'a>(
    root: &nt_hive::KeyNode<'a, &'a [u8]>,
    key_path: &str,
) -> Result<nt_hive::KeyNode<'a, &'a [u8]>> {
    let segments: Vec<&str> = key_path.split('\\').filter(|s| !s.is_empty()).collect();

    match root.subpath(key_path) {
        Some(Ok(node)) => Ok(node),
        Some(Err(e)) => Err(anyhow::Error::new(e)
            .context(format!("failed to parse subpath `{}`", key_path))),
        None => Err(anyhow::anyhow!(
            "no such key path `{}` (segments: {})",
            key_path,
            segments.join(" / ")
        )),
    }
}

/// Collect the direct children of `target` along with their per-child
/// `subkey_count` and `value_count`. Counts are best-effort: a malformed
/// subkey index or value list is logged and counted as 0 rather than
/// aborting the whole listing.
fn list_entries<'a>(
    target: &nt_hive::KeyNode<'a, &'a [u8]>,
) -> Result<Vec<ListEntry>> {
    let children = match target.subkeys() {
        Some(Ok(iter)) => iter,
        Some(Err(e)) => return Err(anyhow::Error::new(e).context("malformed subkey index")),
        None => return Ok(Vec::new()),
    };

    let mut out = Vec::new();
    for child_result in children {
        let child = child_result
            .map_err(|e| anyhow::Error::new(e).context("failed to advance subkey iterator"))?;
        let name = child
            .name()
            .map_err(|e| anyhow::Error::new(e).context("failed to read subkey name"))?
            .to_string_lossy();

        let subkey_count = match child.subkeys() {
            Some(Ok(iter)) => iter.count(),
            _ => 0,
        };
        let value_count = match child.values() {
            Some(Ok(iter)) => iter.count(),
            _ => 0,
        };

        out.push(ListEntry {
            name,
            subkey_count,
            value_count,
        });
    }
    Ok(out)
}

fn render_list_human(stats: &ListStats) -> Result<()> {
    println!("File:    {}", stats.base.path);
    println!("At:      {}", stats.at);
    println!();

    // Use a fixed-width header for predictability across hive shapes.
    println!("{:<40}  {:>8}  {:>8}", "Name", "Subkeys", "Values");
    println!("{}", "─".repeat(60));
    for entry in &stats.entries {
        // Truncate over-long names with a trailing `…` to keep alignment.
        let display_name = truncate_with_ellipsis(&entry.name, 40);
        println!(
            "{:<40}  {:>8}  {:>8}",
            display_name, entry.subkey_count, entry.value_count
        );
    }
    println!();
    println!("Total: {} keys", stats.total_entries);
    Ok(())
}

fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else {
        let keep = max_chars.saturating_sub(1);
        let truncated: String = s.chars().take(keep).collect();
        format!("{truncated}…")
    }
}

fn render_list_json(stats: &ListStats) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(stats)?);
    Ok(())
}

// ----------------------------------------------------------------------
// show
// ----------------------------------------------------------------------

/// A single value of a hive key, post-decoded into the structures we
/// render. `reg_type` is the on-disk REG_* code as a readable string;
/// `data_human` is pre-rendered text; `data_json` is the structured
/// counterpart used by the JSON output sink.
#[derive(Debug, Serialize)]
struct ValueEntry {
    name: String,
    reg_type: String,
    data_human: String,
    data_json: serde_json::Value,
}

/// Payload for the `show` subcommand.
#[derive(Debug, Serialize)]
struct ShowStats {
    #[serde(flatten)]
    base: Stats,
    at: String,
    entries: Vec<ValueEntry>,
    total_values: usize,
}

fn show_command(path: &Path, key_path: Option<&str>, format: OutputFormat) -> Result<()> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read hive file `{}`", path.display()))?;
    let hive = Hive::new(bytes.as_slice()).context("input is not a valid Windows registry hive")?;
    let root = hive
        .root_key_node()
        .context("hive has no root key node")?;

    let target = match key_path {
        None | Some("") | Some(".") => root,
        Some(p) => find_subpath(&root, p)?,
    };
    let target_name: &str = match key_path {
        None | Some("") | Some(".") => "<root>",
        Some(p) => p,
    };

    let entries = read_values(&target)?;

    let stats = ShowStats {
        base: Stats {
            path: path.display().to_string(),
            file_size_bytes: std::fs::metadata(path)
                .context("failed to stat hive file")?
                .len(),
            parsed_ok: true,
            minor_version_known: false,
        },
        at: target_name.to_string(),
        total_values: entries.len(),
        entries,
    };

    match format {
        OutputFormat::Human => render_show_human(&stats),
        OutputFormat::Json => render_show_json(&stats),
    }
}

/// Read every value of `target` and decode name + data according to type.
/// On a partial decode failure, we still return *something* rather than
/// aborting the whole listing — a single corrupt value should not take
/// down the rest.
fn read_values<'a>(
    target: &nt_hive::KeyNode<'a, &'a [u8]>,
) -> Result<Vec<ValueEntry>> {
    let iter = match target.values() {
        Some(Ok(iter)) => iter,
        Some(Err(e)) => return Err(anyhow::Error::new(e).context("malformed value list")),
        None => return Ok(Vec::new()),
    };

    let mut out = Vec::new();
    for val_result in iter {
        let val = val_result
            .map_err(|e| anyhow::Error::new(e).context("failed to advance value iterator"))?;
        let name = val
            .name()
            .map_err(|e| anyhow::Error::new(e).context("failed to read value name"))?
            .to_string_lossy();
        let name = if name.is_empty() { "<default>".to_string() } else { name };

        let reg_type = match val.data_type() {
            Ok(t) => reg_type_label(t).to_string(),
            Err(_) => "REG_UNKNOWN".to_string(),
        };

        let (data_human, data_json) = format_value_data(&val, &reg_type);
        out.push(ValueEntry { name, reg_type, data_human, data_json });
    }
    Ok(out)
}

/// Decode a key value into a textual representation AND a structured
/// JSON value. The textual one is used by the human sink, the structured
/// one by the JSON sink. Both are produced from a single read of the
/// hive so a corrupt big-data cell only gets walked once.
fn format_value_data<'a>(
    val: &nt_hive::KeyValue<'a, &'a [u8]>,
    reg_type: &str,
) -> (String, serde_json::Value) {
    match reg_type {
        "REG_SZ" | "REG_EXPAND_SZ" => match val.string_data() {
            Ok(s) => decode_string_value(&s),
            Err(e) => decode_failure(&e),
        },
        "REG_DWORD" => match val.dword_data() {
            Ok(n) => decode_dword(n),
            Err(e) => decode_failure(&e),
        },
        "REG_DWORD_BIG_ENDIAN" => match val.dword_data() {
            Ok(n) => decode_dword(n),
            Err(e) => decode_failure(&e),
        },
        "REG_QWORD" => match val.qword_data() {
            Ok(n) => decode_qword(n),
            Err(e) => decode_failure(&e),
        },
        "REG_MULTI_SZ" => match val.multi_string_data() {
            Ok(iter) => decode_multi_string(iter),
            Err(e) => decode_failure(&e),
        },
        // REG_BINARY, REG_NONE, REG_LINK, REG_RESOURCE_LIST,
        // REG_FULL_RESOURCE_DESCRIPTOR, REG_RESOURCE_REQUIREMENTS_LIST,
        // plus unknown future codes.
        _ => decode_raw_bytes(val),
    }
}

/// Decode a UTF-16-LE (lossy) string into both a human-readable text
/// (with trailing-NUL trim and `…` truncation past 80 chars) and a JSON
/// string.
fn decode_string_value(s: &str) -> (String, serde_json::Value) {
    const MAX_CHARS: usize = 80;
    let trimmed = s.trim_end_matches('\0').to_string();
    let display = truncate_with_ellipsis(&trimmed, MAX_CHARS);
    let json = serde_json::Value::String(trimmed);
    (display, json)
}

fn decode_dword(n: u32) -> (String, serde_json::Value) {
    let text = format!("{} (0x{:08x})", n, n);
    let json = serde_json::Value::Number(serde_json::Number::from(n));
    (text, json)
}

fn decode_qword(n: u64) -> (String, serde_json::Value) {
    let text = format!("{} (0x{:016x})", n, n);
    let json = serde_json::Value::Number(serde_json::Number::from(n));
    (text, json)
}

fn decode_multi_string<'a>(
    iter: nt_hive::RegMultiSZStrings<'a, &'a [u8]>,
) -> (String, serde_json::Value) {
    let mut lines: Vec<String> = Vec::new();
    for r in iter {
        match r {
            Ok(s) => lines.push(s.trim_end_matches('\0').to_string()),
            Err(e) => {
                return (
                    format!("<multi-sz decode error: {e:?}>"),
                    serde_json::Value::String(format!("<decode error>")),
                );
            }
        }
    }
    let json = serde_json::Value::Array(
        lines
            .iter()
            .map(|s| serde_json::Value::String(s.clone()))
            .collect(),
    );
    let display = if lines.is_empty() {
        "<empty>".to_string()
    } else {
        lines.join("  |  ")
    };
    (truncate_with_ellipsis(&display, 80), json)
}

/// Render raw bytes (REG_BINARY / REG_NONE / unknown) as a hex dump
/// (first 32 bytes shown) plus a JSON byte-array of every byte.
fn decode_raw_bytes<'a>(val: &nt_hive::KeyValue<'a, &'a [u8]>) -> (String, serde_json::Value) {
    // Walk the data iterator once to collect full bytes — we use the same
    // bytes for both the JSON array and the truncated hex dump.
    let bytes: Vec<u8> = match val.data() {
        Ok(nt_hive::KeyValueData::Small(d)) => d.to_vec(),
        Ok(nt_hive::KeyValueData::Big(iter)) => {
            let mut v = Vec::new();
            for slice in iter {
                match slice {
                    Ok(s) => v.extend_from_slice(s),
                    Err(_e) => break,
                }
            }
            v
        }
        Err(_) => Vec::new(),
    };

    const SHOW_BYTES: usize = 32;
    let (preview_text, more) = if bytes.len() <= SHOW_BYTES {
        (hex_dump(&bytes), None)
    } else {
        (
            hex_dump(&bytes[..SHOW_BYTES]),
            Some((bytes.len() - SHOW_BYTES, format!("… ({} more bytes)", bytes.len() - SHOW_BYTES))),
        )
    };
    let preview_text = if let Some((_, suffix)) = more {
        format!("{preview_text} {suffix}")
    } else {
        preview_text
    };

    let json = serde_json::Value::Array(
        bytes
            .iter()
            .map(|b| serde_json::Value::Number(serde_json::Number::from(u64::from(*b))))
            .collect(),
    );

    (preview_text, json)
}

fn decode_failure(e: &nt_hive::NtHiveError) -> (String, serde_json::Value) {
    (
        format!("<decode error: {e:?}>"),
        serde_json::Value::String(format!("<decode error>")),
    )
}

fn hex_dump(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Map an `nt_hive::KeyValueDataType` to its user-facing REG_* name.
fn reg_type_label(t: nt_hive::KeyValueDataType) -> &'static str {
    use nt_hive::KeyValueDataType as K;
    match t {
        K::RegNone => "REG_NONE",
        K::RegSZ => "REG_SZ",
        K::RegExpandSZ => "REG_EXPAND_SZ",
        K::RegBinary => "REG_BINARY",
        K::RegDWord => "REG_DWORD",
        K::RegDWordBigEndian => "REG_DWORD_BIG_ENDIAN",
        K::RegLink => "REG_LINK",
        K::RegMultiSZ => "REG_MULTI_SZ",
        K::RegResourceList => "REG_RESOURCE_LIST",
        K::RegFullResourceDescriptor => "REG_FULL_RESOURCE_DESCRIPTOR",
        K::RegResourceRequirementsList => "REG_RESOURCE_REQUIREMENTS_LIST",
        K::RegQWord => "REG_QWORD",
    }
}

fn render_show_human(stats: &ShowStats) -> Result<()> {
    println!("File:    {}", stats.base.path);
    println!("At:      {}", stats.at);
    println!();
    println!(
        "{:<32}  {:<24}  {}",
        "Name", "Type", "Data"
    );
    println!("{}", "─".repeat(86));
    for entry in &stats.entries {
        println!(
            "{:<32}  {:<24}  {}",
            truncate_with_ellipsis(&entry.name, 32),
            entry.reg_type,
            entry.data_human
        );
    }
    println!();
    println!("Total: {} values", stats.total_values);
    Ok(())
}

fn render_show_json(stats: &ShowStats) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(stats)?);
    Ok(())
}

// ----------------------------------------------------------------------
// find
// ----------------------------------------------------------------------

/// Substring patterns supplied to `find`. Both the `name` and `value`
/// patterns are substring matches (any-of for `name`, single for
/// `value`).
///
/// Matching strategy is intentionally narrow in v1:
///   * Plain substring, no regex (avoids pulling in the `regex` crate
///     and the compile-once surface complexity it brings);
///   * Case sensitivity is a single global flag — per-pattern flags
///     would be ergonomic sugar but add API surface without buying
///     much for a CLI;
///   * REG_BINARY / REG_NONE / REG_RESOURCE_* values are matched
///     against their **stringified hex dump** (the same text you'd
///     see in `show`'s human output) rather than their raw bytes. This
///     is good enough for typical searches and keeps the walker
///     allocation-light.
#[derive(Debug, Clone, Default, Serialize)]
struct FindPatterns {
    name: Vec<String>,
    value: Option<String>,
    case_sensitive: bool,
}

impl FindPatterns {
    fn matches_name(&self, haystack: &str) -> bool {
        if self.name.is_empty() {
            // No name filter → any name passes.
            return true;
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
        // approximate but matches common registry-name patterns in our
        // test data.
        haystack.to_lowercase().contains(&needle.to_lowercase())
    }
}

/// A single match — a key whose name matched (or whose value matched if
/// the user asked for value matching). `matched_values` is non-empty
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

fn find_command(
    path: &Path,
    name: &[String],
    value: Option<&str>,
    case_sensitive: bool,
    max_depth: Option<usize>,
    format: OutputFormat,
) -> Result<()> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read hive file `{}`", path.display()))?;
    let hive = Hive::new(bytes.as_slice()).context("input is not a valid Windows registry hive")?;
    let root = hive
        .root_key_node()
        .context("hive has no root key node")?;

    let patterns = FindPatterns {
        name: name.to_vec(),
        value: value.map(|s| s.to_string()),
        case_sensitive,
    };

    let mut matches = Vec::new();
    let mut total_keys: usize = 0;
    walk_for_matches(
        &root,
        "<root>",
        0,
        max_depth,
        &patterns,
        &mut matches,
        &mut total_keys,
    )?;

    let stats = FindStats {
        base: Stats {
            path: path.display().to_string(),
            file_size_bytes: std::fs::metadata(path)
                .context("failed to stat hive file")?
                .len(),
            parsed_ok: true,
            minor_version_known: false,
        },
        patterns,
        max_depth,
        matches,
        total_keys,
    };

    match format {
        OutputFormat::Human => render_find_human(&stats),
        OutputFormat::Json => render_find_json(&stats),
    }
}

/// Recursively walk the hive, descending only while `depth_limit`
/// permits. Records a match whenever the key's name passes the name
/// filter OR at least one of its values passes the value filter.
fn walk_for_matches<'a>(
    node: &nt_hive::KeyNode<'a, &'a [u8]>,
    path_label: &str,
    depth: usize,
    depth_limit: Option<usize>,
    patterns: &FindPatterns,
    out: &mut Vec<KeyMatch>,
    total_keys: &mut usize,
) -> Result<()> {
    *total_keys += 1;

    let name = node
        .name()
        .map_err(|e| anyhow::Error::new(e).context("failed to read key name during find"))?
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
        if let Some(Ok(iter)) = node.values() {
            for val_result in iter {
                let val = match val_result {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let val_name = match val.name() {
                    Ok(n) => n.to_string_lossy(),
                    Err(_) => continue,
                };
                let reg_type = match val.data_type() {
                    Ok(t) => reg_type_label(t).to_string(),
                    Err(_) => "REG_UNKNOWN".to_string(),
                };
                let (data_human, _) = format_value_data(&val, &reg_type);

                if patterns.matches_value(&val_name)
                    || patterns.matches_value(&data_human)
                {
                    matched_values.push(ValueMatchHint {
                        name: val_name,
                        reg_type,
                        preview: data_human,
                    });
                }
            }
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

    // Decend only while we're within the depth budget. depth_limit=0
    // means: show only root, do not descend.
    if matches!(depth_limit, Some(limit) if depth >= limit) {
        return Ok(());
    }

    if let Some(Ok(iter)) = node.subkeys() {
        for child_result in iter {
            let child = match child_result {
                Ok(c) => c,
                Err(_) => continue,
            };
            let child_name = match child.name() {
                Ok(n) => n.to_string_lossy(),
                Err(_) => continue,
            };
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

    Ok(())
}

fn render_find_human(stats: &FindStats) -> Result<()> {
    println!("File:     {}", stats.base.path);
    println!(
        "Patterns: name~={:?}  value~={:?}  case_sensitive={}",
        stats.patterns.name,
        stats.patterns.value,
        stats.patterns.case_sensitive
    );
    if let Some(limit) = stats.max_depth {
        println!("Max depth: {limit}");
    } else {
        println!("Max depth: unlimited");
    }
    println!(
        "Scanned {} keys, matched {} key(s).",
        stats.total_keys, stats.matches.len()
    );

    if stats.matches.is_empty() {
        return Ok(());
    }
    println!();
    for m in &stats.matches {
        let indent = "  ".repeat(m.depth);
        println!("{indent}{}", m.key_path);
        for v in &m.matched_values {
            println!(
                "{}    • {}: {} = {}",
                indent, v.name, v.reg_type, v.preview
            );
        }
    }
    Ok(())
}

fn render_find_json(stats: &FindStats) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(stats)?);
    Ok(())
}
