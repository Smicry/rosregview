//! Registry-path navigation helper shared by the `list`, `show`, and
//! `find` subcommands.
//!
//! String-truncation helpers (`truncate_with_ellipsis`) used to live
//! here too, but have moved to [`crate::output`] — they are output-
//! formatting concerns, not hive-path concerns.

use anyhow::Result;
use nt_hive::KeyNode;

/// Locate a subkey under `root` given a backslash-separated path.
/// Returns the `KeyNode` at the end of the path, or an error if no such
/// path exists in the hive.
///
/// We rely on `nt_hive::KeyNode::subpath` which performs the descent
/// internally — no manual segment traversal on our side.
pub fn find_subpath<'a>(
    root: &KeyNode<'a, &'a [u8]>,
    key_path: &str,
) -> Result<KeyNode<'a, &'a [u8]>> {
    match root.subpath(key_path) {
        Some(Ok(node)) => Ok(node),
        Some(Err(e)) => Err(crate::error::wrap_hive_error_owned(
            e,
            format!("failed to parse subpath `{key_path}`"),
        )),
        None => {
            // Only split into segments on the error path — the happy
            // path (subpath found) never needs them, so we avoid the
            // Vec allocation for every successful lookup.
            let segments: Vec<&str> = key_path.split('\\').filter(|s| !s.is_empty()).collect();
            Err(anyhow::anyhow!(
                "no such key path `{}` (segments: {})",
                key_path,
                segments.join(" / ")
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_subpath_error_for_missing_key_mentions_segments() {
        // We don't have a real hive to test against here (the integration
        // tests cover that). We do verify the segment splitting logic by
        // checking the error message format on a path we *know* doesn't
        // exist.
        //
        // To exercise the function without a hive we build a minimal
        // synthetic test: skip if we can't load the fixture.
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join("testhive");
        if !p.is_file() {
            eprintln!("skipping: {} not found", p.display());
            return;
        }
        let bytes = std::fs::read(&p).unwrap();
        let hive = nt_hive::Hive::new(bytes.as_slice()).unwrap();
        let root = hive.root_key_node().unwrap();

        // `KeyNode` doesn't impl `Debug`, so we can't `unwrap_err()`.
        // Match instead.
        let err = match find_subpath(&root, "definitely\\missing\\path") {
            Ok(_) => panic!("missing subpath must error"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(msg.contains("no such key path"), "got: {msg}");
        assert!(msg.contains("definitely"), "got: {msg}");
        // The segments are joined with " / ".
        assert!(msg.contains("definitely / missing / path"), "got: {msg}");
    }
}
