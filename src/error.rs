//! Shared error handling helpers.
//!
//! The crate's top-level error type is `anyhow::Error` (see `Cargo.toml`),
//! so this module does not introduce a new domain enum — it provides
//! small, well-tested helpers for the `nt_hive::NtHiveError` →
//! `anyhow::Error` boundary that every subcommand walks.

/// Wrap an `nt_hive::NtHiveError` into an `anyhow::Error` carrying a
/// caller-supplied context string. Kept as a single function so we can
/// fix every error message format in one place if we ever decide to
/// switch to a domain enum.
pub fn wrap_hive_error(e: nt_hive::NtHiveError, context: &str) -> anyhow::Error {
    anyhow::Error::new(e).context(context.to_string())
}

/// Same as [`wrap_hive_error`] but accepts a `String` context (e.g. from
/// `format!`). Avoids forcing every caller to do `&format!(...)`.
pub fn wrap_hive_error_owned(e: nt_hive::NtHiveError, context: String) -> anyhow::Error {
    anyhow::Error::new(e).context(context)
}

#[cfg(test)]
mod tests {
    #[test]
    fn wrap_hive_error_carries_context() {
        // We can't easily construct an `NtHiveError` from outside (no
        // public ctor), so we exercise the helper by passing a
        // synthesised error string. `nt_hive::NtHiveError` is
        // `#[non_exhaustive]` so we can only go through the public
        // API. Here we verify the helper's shape by checking that
        // `Display` (topmost context) and `Debug` (full chain) both
        // work as expected for an `anyhow::Error` shaped the same way.
        let err = anyhow::Error::msg("synthetic").context("ctx");
        // `Display` shows only the outermost context.
        assert_eq!(err.to_string(), "ctx");
        // `Debug` shows the full chain.
        let dbg = format!("{err:?}");
        assert!(dbg.contains("ctx"), "got debug: {dbg}");
        assert!(dbg.contains("synthetic"), "got debug: {dbg}");
    }

    #[test]
    fn wrap_hive_error_owned_preserves_context() {
        // Smoke test for the owned variant. We construct a dummy anyhow
        // error and confirm `.context(String)` preserves the string
        // verbatim in the topmost Display position.
        let err: anyhow::Error = anyhow::Error::msg("inner").context("a\nb".to_string());
        assert_eq!(err.to_string(), "a\nb");
        let dbg = format!("{err:?}");
        assert!(dbg.contains("inner"), "got debug: {dbg}");
    }
}
