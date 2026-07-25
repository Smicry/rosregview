//! `show` subcommand — values of a key, decoded by REG_* type.

use crate::cli::OutputFormat;
use crate::error;
use crate::hive::{format, open};
use crate::output::{
    Stats,
    table::truncate_with_ellipsis,
    value::{format_value_data, reg_type_label},
};
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

/// Public entry point for the `show` subcommand.
pub fn run(path: &Path, key_path: Option<&str>, format_flag: OutputFormat) -> Result<()> {
    let (hive, file_size) = open::load_hive(path)?;
    let root = hive
        .root_key_node()
        .map_err(|e| error::wrap_hive_error(e, "hive has no root key node"))?;

    let target = match key_path {
        None | Some("") | Some(".") => root,
        Some(p) => format::find_subpath(&root, p)?,
    };
    let target_name: &str = match key_path {
        None | Some("") | Some(".") => "<root>",
        Some(p) => p,
    };

    let entries = read_values(&target)?;

    let stats = ShowStats {
        base: Stats::from_hive(path, file_size),
        at: target_name.to_string(),
        total_values: entries.len(),
        entries,
    };

    match format_flag {
        OutputFormat::Human => render_human(&stats),
        OutputFormat::Json => render_json(&stats),
    }
}

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

/// Read every value of `target` and decode name + data according to
/// type. On a partial decode failure, we still return *something*
/// rather than aborting the whole listing — a single corrupt value
/// should not take down the rest.
fn read_values<'a>(target: &nt_hive::KeyNode<'a, &'a [u8]>) -> Result<Vec<ValueEntry>> {
    let iter = match target.values() {
        Some(Ok(iter)) => iter,
        Some(Err(e)) => {
            return Err(error::wrap_hive_error_owned(
                e,
                "malformed value list".to_string(),
            ));
        }
        None => return Ok(Vec::new()),
    };

    let mut out = Vec::new();
    for val_result in iter {
        let val = val_result
            .map_err(|e| error::wrap_hive_error(e, "failed to advance value iterator"))?;
        let name = val
            .name()
            .map_err(|e| error::wrap_hive_error(e, "failed to read value name"))?
            .to_string_lossy();
        let name = if name.is_empty() {
            "<default>".to_string()
        } else {
            name
        };

        let reg_type = match val.data_type() {
            Ok(t) => reg_type_label(t).to_string(),
            Err(_) => "REG_UNKNOWN".to_string(),
        };

        let (data_human, data_json) = format_value_data(&val, &reg_type);
        out.push(ValueEntry {
            name,
            reg_type,
            data_human,
            data_json,
        });
    }
    Ok(out)
}

/// Human-readable sink: aligned `Name | Type | Data` columns plus a
/// total footer. Each entry's `data_human` is pre-decoded by
/// `read_values` (via `output::value::format_value_data`), so this
/// function only handles layout — no hive decoding happens here.
fn render_human(stats: &ShowStats) -> Result<()> {
    println!("File:    {}", stats.base.path);
    println!("At:      {}", stats.at);
    println!();
    println!("{:<32}  {:<24}  Data", "Name", "Type");
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

fn render_json(stats: &ShowStats) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(stats)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::value::format_value_data;
    use std::path::PathBuf;

    fn fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join("testhive")
    }

    #[test]
    fn read_values_on_data_test_returns_nine_typed_entries() {
        let p = fixture_path();
        if !p.is_file() {
            eprintln!("skipping: {} not found", p.display());
            return;
        }
        let (hive, _size) = open::load_hive(&p).unwrap();
        let root = hive.root_key_node().unwrap();
        // Navigate root → data-test
        let target = format::find_subpath(&root, "data-test").unwrap();
        let entries = read_values(&target).unwrap();
        assert_eq!(entries.len(), 9);
        let by_name: std::collections::HashMap<String, &ValueEntry> =
            entries.iter().map(|e| (e.name.clone(), e)).collect();
        // REG_SZ value named "reg-sz" decodes to "sz-test"
        let sz = by_name.get("reg-sz").expect("missing reg-sz");
        assert_eq!(sz.reg_type, "REG_SZ");
        assert_eq!(sz.data_json.as_str(), Some("sz-test"));
        // REG_DWORD value 42 is a native JSON number
        let dw = by_name.get("dword").expect("missing dword");
        assert_eq!(dw.reg_type, "REG_DWORD");
        assert_eq!(dw.data_json.as_u64(), Some(42));
        // REG_QWORD max
        let qw = by_name.get("qword").expect("missing qword");
        assert_eq!(qw.reg_type, "REG_QWORD");
        assert_eq!(qw.data_json.as_u64(), Some(u64::MAX));
    }

    #[test]
    fn read_values_promotes_empty_name_to_default_placeholder() {
        // `read_values` promotes any empty value name to the
        // "<default>" placeholder (see the `if name.is_empty()` branch
        // above). The testhive fixture's `data-test` key has no actual
        // (Default) value, so we can't exercise a real (Default)
        // decode here — but we CAN verify the promotion invariant
        // holds: no entry in the output should carry an empty name.
        let p = fixture_path();
        if !p.is_file() {
            eprintln!("skipping: {} not found", p.display());
            return;
        }
        let (hive, _size) = open::load_hive(&p).unwrap();
        let root = hive.root_key_node().unwrap();
        let target = format::find_subpath(&root, "data-test").unwrap();
        let entries = read_values(&target).unwrap();
        for e in &entries {
            assert!(!e.name.is_empty(), "found empty entry name: {e:?}");
        }
    }

    #[test]
    fn format_value_data_handles_unknown_reg_type_as_raw_bytes() {
        // An unknown REG_* code falls through to decode_raw_bytes. We
        // verify the dispatch path via the public API rather than the
        // private helper.
        let p = fixture_path();
        if !p.is_file() {
            eprintln!("skipping: {} not found", p.display());
            return;
        }
        let (hive, _size) = open::load_hive(&p).unwrap();
        let root = hive.root_key_node().unwrap();
        let target = format::find_subpath(&root, "data-test").unwrap();
        for val in target.values().unwrap().unwrap() {
            let v = val.unwrap();
            let (_, json) = format_value_data(&v, "REG_DEFINITELY_NOT_REAL");
            // Whatever the bytes are, we get back a JSON array.
            assert!(json.is_array());
        }
    }
}
