//! `tree` subcommand — recursive key tree.

use crate::cli::OutputFormat;
use crate::error;
use crate::hive::open;
use crate::output::{Stats, escape_control_chars, write_stdout};
use anyhow::Result;
use nt_hive::KeyNode;
use serde::Serialize;
use std::path::Path;

const MAX_TRAVERSAL_DEPTH: usize = 512;

/// Public entry point for the `tree` subcommand.
pub fn run(path: &Path, depth: Option<usize>, format: OutputFormat) -> Result<()> {
    let stats = open::with_hive(path, |hive, file_size| {
        let root = hive
            .root_key_node()
            .map_err(|e| error::wrap_hive_error(e, "hive has no root key node"))?;
        Ok(TreeStats {
            base: Stats::from_hive(path, file_size, hive.minor_version()),
            depth_limit: depth,
            tree: build_tree(&root, "<root>", depth, 0)?,
        })
    })?;

    match format {
        OutputFormat::Human => render_human(&stats),
        OutputFormat::Json => render_json(&stats),
    }
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

/// Recursively walk a `KeyNode` and build a `KeyTreeNode`.
///
/// `depth_limit` semantics: `None` = unlimited; `Some(n)` = stop
/// descending at depth `n` (i.e. n=0 means show only the root, n=1
/// means root + direct children, ...). `current_depth` is the depth of
/// `node` itself.
fn build_tree<'a>(
    node: &KeyNode<'a, &'a [u8]>,
    name: &str,
    depth_limit: Option<usize>,
    current_depth: usize,
) -> Result<KeyTreeNode> {
    if current_depth > MAX_TRAVERSAL_DEPTH {
        anyhow::bail!(
            "registry key nesting exceeds the safety limit of {MAX_TRAVERSAL_DEPTH} levels"
        );
    }
    // Honor the depth limit *before* recursing into children.
    let reached_limit = matches!(depth_limit, Some(limit) if current_depth >= limit);

    let subkeys = if reached_limit {
        Vec::new()
    } else {
        match node.subkeys() {
            Some(Ok(iter)) => iter
                .map(|child_result| -> Result<KeyTreeNode> {
                    let child = child_result.map_err(|e| {
                        error::wrap_hive_error(e, "failed to advance subkey iterator")
                    })?;
                    let child_name = child
                        .name()
                        .map_err(|e| error::wrap_hive_error(e, "failed to read subkey name"))?
                        .to_string_lossy();
                    build_tree(&child, &child_name, depth_limit, current_depth + 1)
                })
                .collect::<Result<Vec<_>>>()?,
            Some(Err(e)) => {
                return Err(error::wrap_hive_error_owned(
                    e,
                    "malformed subkey index".to_string(),
                ));
            }
            None => Vec::new(),
        }
    };

    Ok(KeyTreeNode {
        name: name.to_string(),
        subkeys,
    })
}

fn render_human(stats: &TreeStats) -> Result<()> {
    let mut out = String::new();
    render_tree_to_string(&stats.tree, 0, &mut out);
    write_stdout(|writer| {
        writeln!(
            writer,
            "File:     {}",
            escape_control_chars(&stats.base.path)
        )?;
        writeln!(writer, "Size:     {} bytes", stats.base.file_size_bytes)?;
        if let Some(limit) = stats.depth_limit {
            writeln!(writer, "Depth:    0..={limit}")?;
        } else {
            writeln!(writer, "Depth:    unlimited")?;
        }
        writeln!(writer)?;
        write!(writer, "{out}")?;
        Ok(())
    })
}

fn render_tree_to_string(node: &KeyTreeNode, depth: usize, out: &mut String) {
    out.push_str(&"  ".repeat(depth));
    out.push_str(&escape_control_chars(&node.name));
    out.push('\n');
    for child in &node.subkeys {
        render_tree_to_string(child, depth + 1, out);
    }
}

fn render_json(stats: &TreeStats) -> Result<()> {
    let json = serde_json::to_string_pretty(stats)?;
    write_stdout(|out| {
        writeln!(out, "{json}")?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join("testhive")
    }

    #[test]
    fn build_tree_depth_zero_has_no_children() {
        let p = fixture_path();
        if !p.is_file() {
            eprintln!("skipping: fixture not found");
            return;
        }
        open::with_hive(&p, |hive, _size| {
            let root = hive.root_key_node().unwrap();
            let tree = build_tree(&root, "<root>", Some(0), 0).unwrap();
            assert_eq!(tree.name, "<root>");
            assert!(tree.subkeys.is_empty());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn build_tree_depth_one_yields_direct_children_only() {
        let p = fixture_path();
        if !p.is_file() {
            eprintln!("skipping: fixture not found");
            return;
        }
        open::with_hive(&p, |hive, _size| {
            let root = hive.root_key_node().unwrap();
            let tree = build_tree(&root, "<root>", Some(1), 0).unwrap();
            assert!(!tree.subkeys.is_empty());
            for child in &tree.subkeys {
                assert!(child.subkeys.is_empty());
            }
            Ok(())
        })
        .unwrap();
    }
}
