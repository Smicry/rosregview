//! Value-type aware decoders shared by `show` and `find`.
//!
//! Both subcommands need to:
//!   1. Map an `nt_hive::KeyValueDataType` to its user-facing REG_* name.
//!   2. Decode a `KeyValue`'s data into a human-readable text + a
//!      structured JSON counterpart (used by `show`'s JSON sink and
//!      `find`'s value-pattern matcher).
//!
//! Keeping these here lets both subcommands agree on the shape without
//! one importing the other's private helpers.

use nt_hive::{KeyValue, KeyValueData, KeyValueDataType, RegMultiSZStrings};

use crate::output::hex::hex_dump;
use crate::output::table::truncate_with_ellipsis;

/// Map an `nt_hive::KeyValueDataType` to its user-facing REG_* name.
pub(crate) fn reg_type_label(t: KeyValueDataType) -> &'static str {
    use nt_hive::KeyValueDataType as K;
    match t {
        K::RegNone => "REG_NONE",
        K::RegSZ => "REG_SZ",
        K::RegExpandSZ => "REG_EXPAND_SZ",
        K::RegBinary => "REG_BINARY",
        K::RegDWord => "REG_DWORD",
        K::RegDWordBigEndian => "REG_DWORD_BIG_ENDIAN",
        K::RegLink => "REG_LINK",
        K::RegMultiSZ => "REG_MULTI_SZ",
        K::RegResourceList => "REG_RESOURCE_LIST",
        K::RegFullResourceDescriptor => "REG_FULL_RESOURCE_DESCRIPTOR",
        K::RegResourceRequirementsList => "REG_RESOURCE_REQUIREMENTS_LIST",
        K::RegQWord => "REG_QWORD",
    }
}

/// Decode a key value into a textual representation AND a structured
/// JSON value. The textual one is used by the human sink, the
/// structured one by the JSON sink. Both are produced from a single
/// read of the hive so a corrupt big-data cell only gets walked once.
pub(crate) fn format_value_data<'a>(
    val: &KeyValue<'a, &'a [u8]>,
    reg_type: &str,
) -> (String, serde_json::Value) {
    match reg_type {
        "REG_SZ" | "REG_EXPAND_SZ" => match val.string_data() {
            Ok(s) => decode_string_value(&s),
            Err(e) => decode_failure(&e),
        },
        "REG_DWORD" => match val.dword_data() {
            Ok(n) => decode_dword(n),
            Err(e) => decode_failure(&e),
        },
        "REG_DWORD_BIG_ENDIAN" => match val.dword_data() {
            Ok(n) => decode_dword(n),
            Err(e) => decode_failure(&e),
        },
        "REG_QWORD" => match val.qword_data() {
            Ok(n) => decode_qword(n),
            Err(e) => decode_failure(&e),
        },
        "REG_MULTI_SZ" => match val.multi_string_data() {
            Ok(iter) => decode_multi_string(iter),
            Err(e) => decode_failure(&e),
        },
        // REG_BINARY, REG_NONE, REG_LINK, REG_RESOURCE_LIST,
        // REG_FULL_RESOURCE_DESCRIPTOR, REG_RESOURCE_REQUIREMENTS_LIST,
        // plus unknown future codes.
        _ => decode_raw_bytes(val),
    }
}

/// Decode a UTF-16-LE (lossy) string into both a human-readable text
/// (with trailing-NUL trim and `…` truncation past 80 chars) and a
/// JSON string.
fn decode_string_value(s: &str) -> (String, serde_json::Value) {
    const MAX_CHARS: usize = 80;
    let trimmed = s.trim_end_matches('\0').to_string();
    let display = truncate_with_ellipsis(&trimmed, MAX_CHARS);
    let json = serde_json::Value::String(trimmed);
    (display, json)
}

fn decode_dword(n: u32) -> (String, serde_json::Value) {
    let text = format!("{} (0x{:08x})", n, n);
    let json = serde_json::Value::Number(serde_json::Number::from(n));
    (text, json)
}

fn decode_qword(n: u64) -> (String, serde_json::Value) {
    let text = format!("{} (0x{:016x})", n, n);
    let json = serde_json::Value::Number(serde_json::Number::from(n));
    (text, json)
}

fn decode_multi_string<'a>(iter: RegMultiSZStrings<'a, &'a [u8]>) -> (String, serde_json::Value) {
    let mut lines: Vec<String> = Vec::new();
    for r in iter {
        match r {
            Ok(s) => lines.push(s.trim_end_matches('\0').to_string()),
            Err(e) => {
                return (
                    format!("<multi-sz decode error: {e:?}>"),
                    serde_json::Value::String("<decode error>".to_string()),
                );
            }
        }
    }
    let json = serde_json::Value::Array(
        lines
            .iter()
            .map(|s| serde_json::Value::String(s.clone()))
            .collect(),
    );
    let display = if lines.is_empty() {
        "<empty>".to_string()
    } else {
        lines.join("  |  ")
    };
    (truncate_with_ellipsis(&display, 80), json)
}

/// Render raw bytes (REG_BINARY / REG_NONE / unknown) as a hex dump
/// (first 32 bytes shown) plus a JSON byte-array of every byte.
fn decode_raw_bytes<'a>(val: &KeyValue<'a, &'a [u8]>) -> (String, serde_json::Value) {
    // Walk the data iterator once to collect full bytes — we use the
    // same bytes for both the JSON array and the truncated hex dump.
    let bytes: Vec<u8> = match val.data() {
        Ok(KeyValueData::Small(d)) => d.to_vec(),
        Ok(KeyValueData::Big(iter)) => {
            let mut v = Vec::new();
            for slice in iter {
                match slice {
                    Ok(s) => v.extend_from_slice(s),
                    Err(_e) => break,
                }
            }
            v
        }
        Err(_) => Vec::new(),
    };

    const SHOW_BYTES: usize = 32;
    let (preview_text, more) = if bytes.len() <= SHOW_BYTES {
        (hex_dump(&bytes), None)
    } else {
        (
            hex_dump(&bytes[..SHOW_BYTES]),
            Some((
                bytes.len() - SHOW_BYTES,
                format!("… ({} more bytes)", bytes.len() - SHOW_BYTES),
            )),
        )
    };
    let preview_text = if let Some((_, suffix)) = more {
        format!("{preview_text} {suffix}")
    } else {
        preview_text
    };

    let json = serde_json::Value::Array(
        bytes
            .iter()
            .map(|b| serde_json::Value::Number(serde_json::Number::from(u64::from(*b))))
            .collect(),
    );

    (preview_text, json)
}

fn decode_failure(e: &nt_hive::NtHiveError) -> (String, serde_json::Value) {
    (
        format!("<decode error: {e:?}>"),
        serde_json::Value::String("<decode error>".to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reg_type_label_covers_all_nt_hive_variants() {
        use nt_hive::KeyValueDataType::*;
        // Every public variant must produce a non-empty string. If
        // nt-hive adds a new variant we'll have a non-exhaustive
        // match error here pointing at this list — that's the signal
        // to update this function.
        let labels = [
            (RegNone, "REG_NONE"),
            (RegSZ, "REG_SZ"),
            (RegExpandSZ, "REG_EXPAND_SZ"),
            (RegBinary, "REG_BINARY"),
            (RegDWord, "REG_DWORD"),
            (RegDWordBigEndian, "REG_DWORD_BIG_ENDIAN"),
            (RegLink, "REG_LINK"),
            (RegMultiSZ, "REG_MULTI_SZ"),
            (RegResourceList, "REG_RESOURCE_LIST"),
            (RegFullResourceDescriptor, "REG_FULL_RESOURCE_DESCRIPTOR"),
            (
                RegResourceRequirementsList,
                "REG_RESOURCE_REQUIREMENTS_LIST",
            ),
            (RegQWord, "REG_QWORD"),
        ];
        for (variant, expected) in labels {
            assert_eq!(reg_type_label(variant), expected);
        }
    }
}
