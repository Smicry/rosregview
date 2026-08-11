//! Read a hive file from disk and construct an `nt_hive::Hive`.
//!
//! Every subcommand starts by calling [`with_hive`]. Centralising the
//! "read + parse" sequence gives us one place to attach file-size
//! collection, error context, and any future checksum/sanity checks.
//!
use anyhow::{Context, Result, bail};
use nt_hive::Hive;
use std::{io::Read, path::Path};

/// Refuse implausibly large inputs before allocating memory for them.
pub const MAX_HIVE_SIZE: u64 = 1024 * 1024 * 1024;

/// Read and parse `path`, then invoke `f` while the backing bytes remain alive.
/// The callback shape avoids leaking the buffer to manufacture a `'static`
/// lifetime for the borrowing `nt_hive::Hive` type.
pub fn with_hive<T>(path: &Path, f: impl FnOnce(&Hive<&[u8]>, u64) -> Result<T>) -> Result<T> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to read hive file `{}`", path.display()))?;
    let file_size = file
        .metadata()
        .with_context(|| format!("failed to stat hive file `{}`", path.display()))?
        .len();
    if file_size > MAX_HIVE_SIZE {
        bail!(
            "hive file `{}` is too large ({} bytes; limit is {} bytes)",
            path.display(),
            file_size,
            MAX_HIVE_SIZE
        );
    }

    let mut bytes = Vec::with_capacity(file_size as usize);
    file.take(MAX_HIVE_SIZE + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("failed to read hive file `{}`", path.display()))?;
    if bytes.len() as u64 > MAX_HIVE_SIZE {
        bail!("hive file `{}` grew beyond the size limit", path.display());
    }

    let hive = Hive::new(bytes.as_slice())
        .with_context(|| format!("`{}` is not a valid Windows registry hive", path.display()))?;
    f(&hive, bytes.len() as u64)
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
    fn with_hive_succeeds_on_real_fixture() {
        let p = fixture_path();
        if !p.is_file() {
            eprintln!("skipping: {} not found", p.display());
            return;
        }
        with_hive(&p, |hive, size| {
            let root = hive.root_key_node().expect("real test hive must have root");
            assert_eq!(size, 159744);
            match root.subkeys() {
                Some(Ok(iter)) => assert!(iter.count() > 0),
                _ => panic!("expected some root subkeys"),
            }
            Ok(())
        })
        .expect("real test hive must parse");
    }

    #[test]
    fn with_hive_rejects_missing_file_with_clear_context() {
        let bogus = std::path::Path::new("/this/path/should/never/exist.hiv");
        // `Hive` doesn't impl `Debug`, so we can't use `expect_err`
        // (which requires `T: Debug` on the Ok arm). Match instead.
        let err = match with_hive(bogus, |_hive, _size| Ok(())) {
            Ok(_) => panic!("missing file must error"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(msg.contains("failed to read hive file"), "got: {msg}");
        assert!(msg.contains("never/exist.hiv"), "got: {msg}");
    }

    #[test]
    fn with_hive_rejects_non_hive_bytes() {
        let tmp =
            std::env::temp_dir().join(format!("rosregview-not-a-hive-{}.bin", std::process::id()));
        let mut f = std::fs::File::create(&tmp).expect("create tmp");
        f.write_all(b"not a hive, just plain text\n")
            .expect("write tmp");
        f.sync_all().ok();

        let err = match with_hive(&tmp, |_hive, _size| Ok(())) {
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
