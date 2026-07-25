//! Registry-path navigation and small string utilities shared by the
//! `list`, `show`, and `find` subcommands.

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
    let segments: Vec<&str> = key_path.split('\\').filter(|s| !s.is_empty()).collect();

    match root.subpath(key_path) {
        Some(Ok(node)) => Ok(node),
        Some(Err(e)) => Err(crate::error::wrap_hive_error_owned(
            e,
            format!("failed to parse subpath `{key_path}`"),
        )),
        None => Err(anyhow::anyhow!(
            "no such key path `{}` (segments: {})",
            key_path,
            segments.join(" / ")
        )),
    }
}

/// Truncate a string to `max_chars` characters, appending `…` if it was
/// longer. Counts by Unicode scalar value (Rust `char`), not bytes.
///
/// Examples (where `max_chars = 5`):
///   * `"hello"`     → `"hello"`
///   * `"hello!!"`   → `"hell…"`
///   * `"héllo"`     → `"héllo"`  (5 chars, even though 6 bytes)
pub fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else {
        let keep = max_chars.saturating_sub(1);
        let truncated: String = s.chars().take(keep).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
        assert_eq!(truncate_with_ellipsis("hi", 5), "hi");
    }

    #[test]
    fn truncate_long_string_with_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello world", 5), "hell…");
    }

    #[test]
    fn truncate_empty_string_is_empty() {
        assert_eq!(truncate_with_ellipsis("", 5), "");
    }

    #[test]
    fn truncate_counts_chars_not_bytes() {
        // `héllo` is 5 chars (6 bytes). max_chars=5 should NOT truncate.
        assert_eq!(truncate_with_ellipsis("héllo", 5), "héllo");
        // 6 chars: h, é, l, l, o, !. Keep first 4 + ellipsis → "héll…".
        assert_eq!(truncate_with_ellipsis("héllo!", 5), "héll…");
        // Pure ASCII sanity.
        assert_eq!(truncate_with_ellipsis("abcdef", 5), "abcd…");
    }

    #[test]
    fn truncate_max_chars_zero_yields_only_ellipsis() {
        // saturating_sub gives 0; we still emit the ellipsis sentinel.
        assert_eq!(truncate_with_ellipsis("anything", 0), "…");
    }

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
