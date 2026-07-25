//! Hex-dump helpers used by `show` for `REG_BINARY` / unknown values.
//!
//! Only the first 32 bytes are dumped to stdout; the rest are summarised
//! by the caller as `… (N more bytes)`.

/// Render `bytes` as space-separated lowercase hex pairs. Caller is
/// responsible for any truncation / size annotation.
pub fn hex_dump(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_dump_empty_is_empty() {
        assert_eq!(hex_dump(&[]), "");
    }

    #[test]
    fn hex_dump_single_byte() {
        assert_eq!(hex_dump(&[0xab]), "ab");
    }

    #[test]
    fn hex_dump_multiple_bytes_with_space_separators() {
        assert_eq!(hex_dump(&[0x01, 0x02, 0x03]), "01 02 03");
    }

    #[test]
    fn hex_dump_lowercases_letters() {
        // Standard `{:02x}` format always uses lowercase hex.
        assert_eq!(hex_dump(&[0xff, 0xfe]), "ff fe");
    }
}
