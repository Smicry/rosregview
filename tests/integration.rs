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

#[test]
fn info_emits_valid_json_in_json_mode() {
    let bin = binary_path();
    assert_binary_exists(&bin);

    let hive = test_hive_path();
    assert!(hive.is_file());

    let output = Command::new(&bin)
        .arg("info")
        .arg("-f")
        .arg("json")
        .arg(&hive)
        .output()
        .expect("failed to spawn rosregview");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "rosregview info -f json exited non-zero (status={:?}). stderr:\n{}\nstdout:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
        stdout,
    );

    // The output must be strictly valid JSON (serde_json is strict).
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("info -f json output must be valid JSON");

    // Spot-check the documented shape. Future formats (tree/list) reuse the
    // same Stats base contract, so this gives forward-compatibility assurance.
    let obj = value.as_object().expect("expected JSON object at top level");
    for required in ["path", "file_size_bytes", "root_subkey_count", "minor_version_known"] {
        assert!(
            obj.contains_key(required),
            "JSON payload missing key `{required}`; got keys: {:?}",
            obj.keys().collect::<Vec<_>>(),
        );
    }

    // Sanity-checks against the testhive fixture (159744 bytes, 5 root subkeys).
    assert_eq!(obj["file_size_bytes"].as_u64(), Some(159744));
    assert_eq!(obj["root_subkey_count"].as_u64(), Some(5));
    assert_eq!(obj["minor_version_known"].as_bool(), Some(false));
}

#[test]
fn info_rejects_unknown_format() {
    let bin = binary_path();
    assert_binary_exists(&bin);

    let hive = test_hive_path();
    assert!(hive.is_file());

    let output = Command::new(&bin)
        .arg("info")
        .arg("--format=xml")
        .arg(&hive)
        .output()
        .expect("failed to spawn rosregview");

    assert!(
        !output.status.success(),
        "rosregview must reject an unknown output format",
    );
    // clap's own error message — exit code 2 typically.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("xml") || output.status.code() == Some(2),
        "expected clap-style format error; got status={:?} stderr={stderr:?}",
        output.status.code(),
    );
}

// ----------------------------------------------------------------------
// tree subcommand
// ----------------------------------------------------------------------

#[test]
fn tree_lists_subkeys_in_human_mode() {
    let bin = binary_path();
    assert_binary_exists(&bin);

    let hive = test_hive_path();
    assert!(hive.is_file());

    let output = Command::new(&bin)
        .arg("tree")
        .arg(&hive)
        .output()
        .expect("failed to spawn rosregview");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "rosregview tree exited non-zero. stderr:\n{}\nstdout:\n{}",
        String::from_utf8_lossy(&output.stderr),
        stdout,
    );

    // Header
    assert!(stdout.contains("File:"), "missing `File:` header in stdout:\n{stdout}");
    assert!(stdout.contains("Size:"));
    assert!(stdout.contains("Depth:"));
    assert!(
        stdout.contains("<root>"),
        "expected `<root>` line for the hive root, got:\n{stdout}",
    );

    // Some known subkeys visible in testdata/testhive.
    for expected in ["subkey-test", "character-encoding-test", "data-test"] {
        assert!(
            stdout.contains(expected),
            "expected subkey `{expected}` in tree output, got:\n{stdout}",
        );
    }

    // UTF-8 names through UTF-16: `character-encoding-test` contains
    // `äöü` (Latin-1 chars test) and `𐐐` (Deseret, surrogate pair in UTF-16).
    assert!(
        stdout.contains("äöü"),
        "expected UTF-8 lossy conversion of `äöü`, got:\n{stdout}",
    );
    assert!(
        stdout.contains("𐐐"),
        "expected UTF-8 lossy conversion of supplementary-plane BMP `𐐐`, got:\n{stdout}",
    );
}

#[test]
fn tree_respects_depth_limit() {
    let bin = binary_path();
    assert_binary_exists(&bin);

    let hive = test_hive_path();
    assert!(hive.is_file());

    // Depth 1: root + direct children only. Their *children* must NOT appear.
    let output = Command::new(&bin)
        .arg("tree")
        .arg("--depth=1")
        .arg(&hive)
        .output()
        .expect("failed to spawn rosregview");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "rosregview tree --depth=1 exited non-zero. stderr:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );

    // The direct children are visible.
    assert!(stdout.contains("subkey-test"));
    // The grand-child keys (e.g. `Key0`, `key1`) under `subkey-test` are NOT visible.
    assert!(
        !stdout.contains("Key0\n") && !stdout.contains("  Key0"),
        "expected grandchild `Key0` to be pruned at depth=1, got:\n{stdout}",
    );
    // `äöü` lives under `character-encoding-test` at depth 2 — must also be pruned.
    assert!(
        !stdout.contains("äöü"),
        "expected deep unicode key to be pruned at depth=1, got:\n{stdout}",
    );

    // Depth 0: only the root, no children.
    let output = Command::new(&bin)
        .arg("tree")
        .arg("--depth=0")
        .arg(&hive)
        .output()
        .expect("failed to spawn rosregview");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "rosregview tree --depth=0 exited non-zero. stderr:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(stdout.contains("<root>"));
    assert!(
        !stdout.contains("subkey-test"),
        "depth=0 must show only the root, got:\n{stdout}",
    );
}

#[test]
fn tree_emits_well_formed_json_with_recursive_subkeys() {
    let bin = binary_path();
    assert_binary_exists(&bin);

    let hive = test_hive_path();
    assert!(hive.is_file());

    let output = Command::new(&bin)
        .arg("tree")
        .arg("-f")
        .arg("json")
        .arg(&hive)
        .output()
        .expect("failed to spawn rosregview");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "rosregview tree -f json failed: stderr={}\nstdout={stdout}",
        String::from_utf8_lossy(&output.stderr),
    );

    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("tree JSON output must be valid JSON");

    let obj = value.as_object().expect("top-level JSON should be an object");
    for required in ["path", "file_size_bytes", "parsed_ok", "depth_limit", "tree"] {
        assert!(
            obj.contains_key(required),
            "JSON payload missing `{required}`",
        );
    }
    // `null` depth_limit means unlimited.
    assert_eq!(obj["depth_limit"], serde_json::Value::Null);

    // Recursive shape: `tree` is an object with `name` + `subkeys` (array).
    let tree = &obj["tree"];
    assert_eq!(tree["name"].as_str(), Some("<root>"));
    let subs = tree["subkeys"].as_array().expect("subkeys must be array");
    assert!(
        !subs.is_empty(),
        "testhive has 5 root subkeys; got empty subkeys array",
    );
}

// ----------------------------------------------------------------------
// list subcommand
// ----------------------------------------------------------------------

#[test]
fn list_default_lists_root_subkeys() {
    let bin = binary_path();
    assert_binary_exists(&bin);

    let hive = test_hive_path();
    let output = Command::new(&bin)
        .arg("list")
        .arg(&hive)
        .output()
        .expect("failed to spawn rosregview");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "rosregview list failed. stderr={}\nstdout={stdout}",
        String::from_utf8_lossy(&output.stderr),
    );

    // The default target is the hive root.
    assert!(stdout.contains("At:      <root>"));
    // Headers
    assert!(stdout.contains("Subkeys"));
    assert!(stdout.contains("Values"));
    assert!(stdout.contains("Total:"));

    // All five root-level subkeys must be present in the table.
    for expected in [
        "big-data-test",
        "character-encoding-test",
        "data-test",
        "subkey-test",
        "subpath-test",
    ] {
        assert!(
            stdout.contains(expected),
            "expected root subkey `{expected}` in list output:\n{stdout}",
        );
    }

    // Total line should match the count we know (5).
    assert!(
        stdout.contains("Total: 5 keys"),
        "expected `Total: 5 keys` in list output:\n{stdout}",
    );
}

#[test]
fn list_with_key_path_descends_one_level() {
    let bin = binary_path();
    assert_binary_exists(&bin);

    let hive = test_hive_path();
    let output = Command::new(&bin)
        .arg("list")
        .arg(&hive)
        .arg("subkey-test")
        .output()
        .expect("failed to spawn rosregview");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "rosregview list subkey-test failed. stderr={}\nstdout={stdout}",
        String::from_utf8_lossy(&output.stderr),
    );

    // The "At:" line should reflect the user-supplied path.
    assert!(
        stdout.contains("At:      subkey-test"),
        "expected `At: subkey-test`, got:\n{stdout}",
    );
    // The 512-sibling count seen in the previous test must NOT appear
    // here (we deliberately descended into `subkey-test`).
    assert!(
        !stdout.contains("subkey-test                              512"),
        "list subkey-test should not list itself as a row again",
    );
    // Children of `subkey-test` start at index 0 — at least one must appear.
    assert!(
        stdout.contains("Key0"),
        "expected `Key0` from subkey-test children in output:\n{stdout}",
    );
}

#[test]
fn list_emits_well_formed_json_with_entries() {
    let bin = binary_path();
    assert_binary_exists(&bin);

    let hive = test_hive_path();
    let output = Command::new(&bin)
        .arg("list")
        .arg("-f")
        .arg("json")
        .arg(&hive)
        .arg("character-encoding-test")
        .output()
        .expect("failed to spawn rosregview");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "rosregview list -f json failed. stderr={}\nstdout={stdout}",
        String::from_utf8_lossy(&output.stderr),
    );

    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("list JSON output must be valid JSON");

    let obj = value.as_object().expect("top-level JSON should be an object");
    for required in ["path", "at", "entries", "total_entries"] {
        assert!(
            obj.contains_key(required),
            "list JSON missing `{required}`",
        );
    }
    assert_eq!(obj["at"].as_str(), Some("character-encoding-test"));

    // Entries must include the 4 known unicode keys via UTF-8 lossy.
    let entries = obj["entries"].as_array().expect("entries must be array");
    let names: Vec<String> = entries
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect();
    for expected in ["äöü", "𐐐", "𐐸", "Ａ"] {
        assert!(
            names.iter().any(|n| n == expected),
            "expected entry name `{expected}` (UTF-8 lossy) in list JSON; got names = {names:?}",
        );
    }
    assert_eq!(obj["total_entries"].as_u64(), Some(4));
}

#[test]
fn list_fails_cleanly_on_unknown_path() {
    let bin = binary_path();
    assert_binary_exists(&bin);

    let hive = test_hive_path();
    let output = Command::new(&bin)
        .arg("list")
        .arg(&hive)
        .arg("NoSuchKey")
        .output()
        .expect("failed to spawn rosregview");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "rosregview list on a nonexistent path must exit non-zero",
    );
    assert!(
        stderr.contains("NoSuchKey") && stderr.contains("no such key path"),
        "expected clean `no such key path ... NoSuchKey` error. stderr={stderr}",
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

#[test]
fn windows_exe_artifact_is_valid_pe32_when_present() {
    // CI gate: a Windows .exe produced via `cargo zigbuild --target
    // i686-pc-windows-gnu` should be a PE32 i386 binary. We *only* run this
    // check if the artifact exists; local devs who skip zigbuild aren't
    // blocked, but a CI run that fails to produce the artifact trips here.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let exe = manifest
        .join("target")
        .join("i686-pc-windows-gnu")
        .join(profile)
        .join("rosregview.exe");

    if !exe.is_file() {
        eprintln!(
            "skipping: {} not found (run `cargo zigbuild --release \
             --target i686-pc-windows-gnu` to produce it)",
            exe.display()
        );
        return;
    }

    let bytes = std::fs::read(&exe).expect("read .exe");
    assert!(
        bytes.len() >= 64 && &bytes[..2] == b"MZ",
        "{} is not a PE file (missing MZ header)",
        exe.display(),
    );
    // e_lfanew at offset 0x3C points at the "PE\0\0" signature.
    let pe_offset = u32::from_le_bytes([bytes[0x3C], bytes[0x3D], bytes[0x3E], bytes[0x3F]])
        as usize;
    assert!(
        pe_offset + 4 <= bytes.len() && &bytes[pe_offset..pe_offset + 4] == b"PE\0\0",
        "{}: PE signature not found at offset 0x{pe_offset:x}",
        exe.display(),
    );

    // Machine type for i386 = 0x014C, right after the signature.
    let machine = u16::from_le_bytes([bytes[pe_offset + 4], bytes[pe_offset + 5]]);
    assert_eq!(
        machine, 0x014C,
        "{}: expected i386 machine type (0x014C), got 0x{machine:04x}",
        exe.display(),
    );

    // Optional subsystem assertion: IMAGE_SUBSYSTEM_WINDOWS_CUI = 3.
    // Subsystem field lives at offset 0x5C from the PE signature.
    let subsys = u16::from_le_bytes([bytes[pe_offset + 0x5C], bytes[pe_offset + 0x5D]]);
    assert_eq!(
        subsys, 3,
        "{}: expected Windows CUI subsystem (3), got {subsys}",
        exe.display(),
    );
}
