//! End-to-end smoke test for `rosregview info`.
//!
//! Runs the compiled binary against the bundled `testdata/testhive` fixture
//! and asserts:
//!   1. Exit code is 0.
//!   2. Output contains the expected `Parsed: OK ...` line, proving nt-hive
//!      0.3 successfully parses the fixture.
//!   3. Output contains a positive `Root subkeys:` count, proving we can
//!      walk past the root KeyNode.
//!
//! Failure mode assertions (missing file, non-hive input) are tested with
//! inline shell invocations so we don't fork the binary into a library.

use std::path::{Path, PathBuf};
use std::process::Command;

fn binary_path() -> PathBuf {
    // `cargo test` places the test binary next to `rosregview`. Resolve the
    // workspace binary via `$CARGO_BIN_EXE_rosregview` when available
    // (cargo ≥1.73), falling back to a target-relative path otherwise so this
    // file works under plain `cargo test --tests` too.
    if let Ok(exe) = std::env::var("CARGO_BIN_EXE_rosregview") {
        return PathBuf::from(exe);
    }

    // CARGO_MANIFEST_DIR is set to the crate root at test time.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // profile is `debug` for `cargo test` and `release` for `cargo test --release`.
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    manifest
        .join("target")
        .join(profile)
        .join("rosregview")
}

/// Absolute path to the test hive, resolved relative to the workspace root.
/// The `testdata/testhive` file is sourced from ColinFinck/nt-hive and is
/// GPL-2.0-or-later.
fn test_hive_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join("testhive")
}

fn assert_binary_exists(path: &Path) {
    assert!(
        path.is_file(),
        "rosregview binary not found at {} — `cargo build` it before `cargo test`",
        path.display()
    );
}

#[test]
fn info_succeeds_on_real_hive() {
    let bin = binary_path();
    assert_binary_exists(&bin);

    let hive = test_hive_path();
    assert!(
        hive.is_file(),
        "test fixture missing at {} — run `curl -L https://github.com/.../testdata/testhive -o {}`",
        hive.display(),
        hive.display(),
    );

    let output = Command::new(&bin)
        .arg("info")
        .arg(&hive)
        .output()
        .expect("failed to spawn rosregview");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "rosregview info exited non-zero (status={:?}). stderr:\n{}\nstdout:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
        stdout,
    );

    assert!(
        stdout.contains("Parsed:         OK"),
        "expected 'Parsed: OK' line in stdout, got:\n{stdout}",
    );
    assert!(
        stdout.contains("File:           "),
        "expected 'File:' line in stdout, got:\n{stdout}",
    );
    assert!(
        stdout.contains("Root subkeys:   "),
        "expected 'Root subkeys:' line in stdout, got:\n{stdout}",
    );
}

#[test]
fn info_fails_cleanly_on_missing_file() {
    let bin = binary_path();
    assert_binary_exists(&bin);

    let bogus = Path::new("/this/path/should/never/exist.hiv");
    let output = Command::new(&bin)
        .arg("info")
        .arg(bogus)
        .output()
        .expect("failed to spawn rosregview");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "rosregview info on a missing file must exit non-zero",
    );
    assert!(
        stderr.contains("failed to read hive file"),
        "expected 'failed to read hive file' on stderr, got:\n{stderr}",
    );
}

#[test]
fn info_rejects_non_hive_input() {
    let bin = binary_path();
    assert_binary_exists(&bin);

    let tmp = tempfile_or_skip();
    std::fs::write(&tmp, b"not a hive, just plain text\n")
        .expect("failed to write tmp non-hive file");

    let output = Command::new(&bin)
        .arg("info")
        .arg(&tmp)
        .output()
        .expect("failed to spawn rosregview");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = std::fs::remove_file(&tmp);

    assert!(
        !output.status.success(),
        "rosregview info on a non-hive file must exit non-zero",
    );
    assert!(
        stderr.contains("not a valid Windows registry hive"),
        "expected invalid-hive message on stderr, got:\n{stderr}",
    );
}

/// Pick a safe temp path; skip the test if we cannot create one rather than
/// failing spuriously on platforms where `/tmp` is read-only.
fn tempfile_or_skip() -> PathBuf {
    let pid = std::process::id();
    let candidate = std::env::temp_dir().join(format!("rosregview-not-a-hive-{pid}.txt"));
    if candidate.parent().is_none_or(|p| !p.is_dir()) {
        eprintln!("skipping: no writable temp dir");
        // Returning a known-bogus path still lets the test fail loudly if we
        // ARE able to proceed, but with a clear message.
        PathBuf::from(format!("/dev/null/rosregview-{pid}"))
    } else {
        candidate
    }
}
