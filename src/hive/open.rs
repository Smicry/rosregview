//! Read a hive file from disk and construct an `nt_hive::Hive`.
//!
//! Every subcommand starts by calling [`load_hive`]. Centralising the
//! "read + parse" sequence gives us one place to attach file-size
//! collection, error context, and any future checksum/sanity checks.
//!
//! ## Memory model
//!
//! `nt_hive::Hive<'_, B>` borrows from its byte buffer, so the buffer
//! must outlive the `Hive`. To make [`load_hive`] return a `Hive` by
//! value we [`Box::leak`] the read bytes into a `&'static [u8]`. The
//! leak is bounded by the number of distinct hive files the process
//! opens in its lifetime — a CLI that reads one hive and exits leaks
//! at most one file's worth of bytes, reclaimed by the OS on exit.
//!
//! [`Box::leak`]: std::boxed::Box::leak

use anyhow::{Context, Result};
use nt_hive::Hive;
use std::path::Path;

/// Read `path` from disk and feed the bytes to `nt_hive::Hive::new`.
/// Returns the parsed [`Hive`] (with a `'static` lifetime thanks to
/// `Box::leak`) together with the file size (taken from the OS stat,
/// not from the buffer — these can differ for hive variants that use
/// sparse-on-disk formats).
pub fn load_hive(path: &Path) -> Result<(Hive<&'static [u8]>, u64)> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read hive file `{}`", path.display()))?;
    let file_size = std::fs::metadata(path)
        .with_context(|| format!("failed to stat hive file `{}`", path.display()))?
        .len();

    let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    let hive = Hive::new(leaked)
        .with_context(|| format!("`{}` is not a valid Windows registry hive", path.display()))?;

    Ok((hive, file_size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Path to the bundled test hive fixture. Used by every test in
    /// this module that needs a real hive to parse.
    fn fixture_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join("testhive")
    }

    #[test]
    fn load_hive_succeeds_on_real_fixture() {
        let p = fixture_path();
        if !p.is_file() {
            eprintln!("skipping: {} not found", p.display());
            return;
        }
        let (hive, size) = load_hive(&p).expect("real test hive must parse");
        let root = hive.root_key_node().expect("real test hive must have root");
        // Sanity: file size is 159744 bytes (per nt-hive's bundled fixture).
        assert_eq!(size, 159744);
        // Sanity: root has at least one subkey (the fixture has 5).
        match root.subkeys() {
            Some(Ok(iter)) => assert!(iter.count() > 0),
            _ => panic!("expected some root subkeys"),
        }
    }

    #[test]
    fn load_hive_rejects_missing_file_with_clear_context() {
        let bogus = std::path::Path::new("/this/path/should/never/exist.hiv");
        // `Hive` doesn't impl `Debug`, so we can't use `expect_err`
        // (which requires `T: Debug` on the Ok arm). Match instead.
        let err = match load_hive(bogus) {
            Ok(_) => panic!("missing file must error"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(msg.contains("failed to read hive file"), "got: {msg}");
        assert!(msg.contains("never/exist.hiv"), "got: {msg}");
    }

    #[test]
    fn load_hive_rejects_non_hive_bytes() {
        let tmp =
            std::env::temp_dir().join(format!("rosregview-not-a-hive-{}.bin", std::process::id()));
        let mut f = std::fs::File::create(&tmp).expect("create tmp");
        f.write_all(b"not a hive, just plain text\n")
            .expect("write tmp");
        f.sync_all().ok();

        let err = match load_hive(&tmp) {
            Ok(_) => panic!("non-hive must error"),
            Err(e) => e,
        };
        let msg = err.to_string();
        let _ = std::fs::remove_file(&tmp);

        assert!(
            msg.contains("not a valid Windows registry hive"),
            "got: {msg}"
        );
    }
}
