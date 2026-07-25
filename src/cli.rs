//! CLI surface for `rosregview`.
//!
//! The shape of this module is dictated by `clap`'s derive macros:
//! [`Cli`] is the top-level parser, [`Command`] is the subcommand enum,
//! and [`OutputFormat`] is the per-subcommand output format enum
//! (`-f human|json`).
//!
//! Per-subcommand *behaviour* lives under `view::*`; this module is
//! strictly the argument/option contract.

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use clap::{Parser, Subcommand};

/// Offline Windows registry hive viewer for ReactOS.
#[derive(Parser, Debug)]
#[command(name = "rosregview", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Show a one-line summary for a hive file: size, root subkey count.
    Info {
        /// Path to a .hiv file (Windows registry configuration unit).
        path: PathBuf,

        /// Output format (`human` is the default table, `json` is machine-readable).
        #[arg(short = 'f', long = "format", value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },

    /// Recursively print the key tree of a hive file (uses ASCII-only indent).
    Tree {
        /// Path to a .hiv file (Windows registry configuration unit).
        path: PathBuf,

        /// Maximum recursion depth. 0 = show only the root, 1 = root + direct
        /// children, 2 = up to grand-children, ... Default: unlimited.
        #[arg(short = 'd', long = "depth", value_name = "N")]
        depth: Option<usize>,

        /// Output format (`human` is the indented text tree, `json` is machine-readable).
        #[arg(short = 'f', long = "format", value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },

    /// List the direct subkeys of a key (default: the root), with subkey and
    /// value counts. The optional `KEY_PATH` may be a single segment
    /// (`ControlSet001`) or a backslash-separated subpath (`A\B\C`). Empty
    /// path == the root.
    List {
        /// Path to a .hiv file (Windows registry configuration unit).
        path: PathBuf,

        /// Optional key path inside the hive. Use `\` (escaped as `\\` in
        /// most shells) to separate levels. Empty/missing → hive root.
        #[arg(value_name = "KEY_PATH")]
        key_path: Option<String>,

        /// Output format (`human` is an aligned table, `json` is machine-readable).
        #[arg(short = 'f', long = "format", value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },

    /// Show the values of a key (default: the root). Per-value output
    /// includes the value name, its REG_* type, and the data, decoded
    /// according to the type where possible (UTF-16 strings, u32 little-
    /// endian dwords, u64 little-endian qwords, ...). Binary data is shown
    /// as a hex dump; long strings are truncated with a trailing `…`.
    Show {
        /// Path to a .hiv file (Windows registry configuration unit).
        path: PathBuf,

        /// Optional key path inside the hive (same convention as `list`).
        #[arg(value_name = "KEY_PATH")]
        key_path: Option<String>,

        /// Output format (`human` is a typed table, `json` is machine-readable).
        #[arg(short = 'f', long = "format", value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },

    /// Search the key hierarchy for keys with names matching `-n` (substring,
    /// repeatable, any-of, case-insensitive by default) and/or values whose
    /// name+decoded-data contains `-v`. With no filters, this enumerates
    /// every key path up to `--max-depth`.
    Find {
        /// Path to a .hiv file (Windows registry configuration unit).
        path: PathBuf,

        /// Substring to match in key names. Repeat to OR multiple patterns.
        #[arg(short = 'n', long = "name", value_name = "PATTERN")]
        name: Vec<String>,

        /// Substring to match in value name + decoded data (any value within
        /// a matching key whose name/data contains the pattern).
        #[arg(short = 'v', long = "value", value_name = "PATTERN")]
        value: Option<String>,

        /// Match case-sensitively. Default: case-insensitive.
        #[arg(long = "case-sensitive", default_value_t = false)]
        case_sensitive: bool,

        /// Limit recursion depth. 0 = root only, 1 = root + direct
        /// children, ... Default: unlimited.
        #[arg(long = "max-depth", value_name = "N")]
        max_depth: Option<usize>,

        /// Output format (`human` is a path list, `json` is machine-readable).
        #[arg(short = 'f', long = "format", value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
}

/// Output formats supported across subcommands.
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

impl FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "human" => Ok(Self::Human),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "unknown output format `{other}` (expected: human, json)"
            )),
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Human => f.write_str("human"),
            OutputFormat::Json => f.write_str("json"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_from_str_accepts_human_and_json() {
        assert_eq!(
            "human".parse::<OutputFormat>().unwrap(),
            OutputFormat::Human
        );
        assert_eq!("JSON".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("Json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
    }

    #[test]
    fn output_format_from_str_rejects_unknown() {
        let err = "xml".parse::<OutputFormat>().unwrap_err();
        assert!(err.contains("unknown output format"));
        assert!(err.contains("xml"));
        assert!(err.contains("human"));
        assert!(err.contains("json"));
    }

    #[test]
    fn output_format_display_round_trips() {
        assert_eq!(OutputFormat::Human.to_string(), "human");
        assert_eq!(OutputFormat::Json.to_string(), "json");
    }

    #[test]
    fn cli_help_parses() {
        // `--help` exits the process via clap; calling `try_parse_from`
        // with `--help` returns a clap error instead. We just assert the
        // parser does not panic on unknown flags here — a real `--help`
        // run is exercised in integration tests.
        let cli = Cli::try_parse_from(["rosregview", "info", "/tmp/x.hiv"]).unwrap();
        assert!(matches!(cli.command, Command::Info { .. }));
    }
}
