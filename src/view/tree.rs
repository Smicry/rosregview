//! `tree` subcommand — recursive key tree.

use crate::cli::OutputFormat;
use crate::error;
use crate::hive::open;
use crate::output::Stats;
use anyhow::Result;
use nt_hive::KeyNode;
use serde::Serialize;
use std::path::Path;

/// Public entry point for the `tree` subcommand.
pub fn run(path: &Path, depth: Option<usize>, format: OutputFormat) -> Result<()> {
    let (hive, file_size) = open::load_hive(path)?;
    let root = hive
        .root_key_node()
        .map_err(|e| error::wrap_hive_error(e, "hive has no root key node"))?;

    let tree = build_tree(&root, "<root>", depth, 0)?;
    let stats = TreeStats {
        base: Stats::from_hive(path, file_size),
        depth_limit: depth,
        tree,
    };

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

fn render_json(stats: &TreeStats) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(stats)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_hive() -> Option<nt_hive::Hive<&'static [u8]>> {
        // We have to leak the bytes to get a 'static lifetime, since
        // the hive borrows from its byte buffer.
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join("testhive");
        if !p.is_file() {
            return None;
        }
        let bytes = std::fs::read(&p).unwrap();
        let boxed: Box<[u8]> = bytes.into_boxed_slice();
        let leaked: &'static [u8] = Box::leak(boxed);
        nt_hive::Hive::new(leaked).ok()
    }

    #[test]
    fn build_tree_depth_zero_has_no_children() {
        let Some(hive) = fixture_hive() else {
            eprintln!("skipping: fixture not found");
            return;
        };
        let root = hive.root_key_node().unwrap();
        let tree = build_tree(&root, "<root>", Some(0), 0).unwrap();
        assert_eq!(tree.name, "<root>");
        assert!(
            tree.subkeys.is_empty(),
            "depth=0 must yield no children, got {:?}",
            tree.subkeys
        );
    }

    #[test]
    fn build_tree_depth_one_yields_direct_children_only() {
        let Some(hive) = fixture_hive() else {
            eprintln!("skipping: fixture not found");
            return;
        };
        let root = hive.root_key_node().unwrap();
        let tree = build_tree(&root, "<root>", Some(1), 0).unwrap();
        assert!(!tree.subkeys.is_empty());
        for child in &tree.subkeys {
            assert!(
                child.subkeys.is_empty(),
                "depth=1 must yield direct children only; got grandchildren under `{}`",
                child.name
            );
        }
    }
}
