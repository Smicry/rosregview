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

use anyhow::Result;
use nt_hive::{KeyValue, KeyValueData, KeyValueDataType, RegMultiSZStrings};

use crate::output::hex::hex_dump;
use crate::output::truncate_with_ellipsis;

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

/// Decode a value into display, search, and optional JSON representations.
/// Raw big-data cells are walked once and are not copied for human output.
pub(crate) struct FormattedValue {
    pub human: String,
    pub json: serde_json::Value,
    pub search: String,
}

pub(crate) fn format_value_data<'a>(
    val: &KeyValue<'a, &'a [u8]>,
    reg_type: &str,
    include_json: bool,
) -> Result<FormattedValue> {
    match reg_type {
        "REG_SZ" | "REG_EXPAND_SZ" => decode_string_value(
            &val.string_data().map_err(anyhow::Error::new)?,
            include_json,
        ),
        "REG_DWORD" | "REG_DWORD_BIG_ENDIAN" => Ok(decode_dword(
            val.dword_data().map_err(anyhow::Error::new)?,
            include_json,
        )),
        "REG_QWORD" => Ok(decode_qword(
            val.qword_data().map_err(anyhow::Error::new)?,
            include_json,
        )),
        "REG_MULTI_SZ" => decode_multi_string(
            val.multi_string_data().map_err(anyhow::Error::new)?,
            include_json,
        ),
        // REG_BINARY, REG_NONE, REG_LINK, REG_RESOURCE_LIST,
        // REG_FULL_RESOURCE_DESCRIPTOR, REG_RESOURCE_REQUIREMENTS_LIST,
        // plus unknown future codes.
        _ => decode_raw_bytes(val, include_json),
    }
}

/// Decode a UTF-16-LE (lossy) string into both a human-readable text
/// (with trailing-NUL trim and `…` truncation past 80 chars) and a
/// JSON string.
fn decode_string_value(s: &str, include_json: bool) -> Result<FormattedValue> {
    const MAX_CHARS: usize = 80;
    let trimmed = s.trim_end_matches('\0').to_string();
    Ok(FormattedValue {
        human: truncate_with_ellipsis(&trimmed, MAX_CHARS),
        json: if include_json {
            serde_json::Value::String(trimmed.clone())
        } else {
            serde_json::Value::Null
        },
        search: trimmed,
    })
}

fn decode_dword(n: u32, include_json: bool) -> FormattedValue {
    let text = format!("{} (0x{:08x})", n, n);
    FormattedValue {
        human: text.clone(),
        json: if include_json {
            n.into()
        } else {
            serde_json::Value::Null
        },
        search: text,
    }
}

fn decode_qword(n: u64, include_json: bool) -> FormattedValue {
    let text = format!("{} (0x{:016x})", n, n);
    FormattedValue {
        human: text.clone(),
        json: if include_json {
            n.into()
        } else {
            serde_json::Value::Null
        },
        search: text,
    }
}

fn decode_multi_string<'a>(
    iter: RegMultiSZStrings<'a, &'a [u8]>,
    include_json: bool,
) -> Result<FormattedValue> {
    let mut lines: Vec<String> = Vec::new();
    for r in iter {
        let s = r.map_err(anyhow::Error::new)?;
        lines.push(s.trim_end_matches('\0').to_string());
    }
    let search = if lines.is_empty() {
        "<empty>".to_string()
    } else {
        lines.join("  |  ")
    };
    let json = if include_json {
        serde_json::Value::Array(lines.into_iter().map(serde_json::Value::String).collect())
    } else {
        serde_json::Value::Null
    };
    Ok(FormattedValue {
        human: truncate_with_ellipsis(&search, 80),
        json,
        search,
    })
}

/// Render raw bytes (REG_BINARY / REG_NONE / unknown) as a hex dump
/// (first 32 bytes shown) plus a JSON byte-array of every byte.
fn decode_raw_bytes<'a>(
    val: &KeyValue<'a, &'a [u8]>,
    include_json: bool,
) -> Result<FormattedValue> {
    const SHOW_BYTES: usize = 32;
    let mut preview = Vec::with_capacity(SHOW_BYTES);
    let mut total = 0usize;
    let mut json_bytes = include_json.then(Vec::new);
    let mut consume = |bytes: &[u8]| {
        total = total.saturating_add(bytes.len());
        let remaining = SHOW_BYTES.saturating_sub(preview.len());
        preview.extend_from_slice(&bytes[..bytes.len().min(remaining)]);
        if let Some(values) = &mut json_bytes {
            values.extend(bytes.iter().copied().map(serde_json::Value::from));
        }
    };
    match val.data().map_err(anyhow::Error::new)? {
        KeyValueData::Small(bytes) => consume(bytes),
        KeyValueData::Big(iter) => {
            for slice in iter {
                consume(slice.map_err(anyhow::Error::new)?);
            }
        }
    }

    let preview_text = if total <= SHOW_BYTES {
        hex_dump(&preview)
    } else {
        format!(
            "{} … ({} more bytes)",
            hex_dump(&preview),
            total - SHOW_BYTES
        )
    };
    Ok(FormattedValue {
        search: preview_text.clone(),
        human: preview_text,
        json: json_bytes
            .map(serde_json::Value::Array)
            .unwrap_or(serde_json::Value::Null),
    })
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

    #[test]
    fn string_search_text_is_not_truncated_with_display() {
        let input = format!("{}needle", "x".repeat(100));
        let formatted = decode_string_value(&input, false).unwrap();
        assert!(!formatted.human.contains("needle"));
        assert!(formatted.search.contains("needle"));
        assert!(formatted.json.is_null());
    }
}
