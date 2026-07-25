//! rosregview — Offline Windows registry hive viewer for ReactOS.
//!
//! See `Cargo.toml` for project metadata and `README.md` for usage.
//!
//! Subcommands currently implemented:
//!   * `info <hive>`                  — overview + JSON via `-f json`
//!   * `tree <hive> [--depth N]`      — recursive key tree, also JSON
//!   * `list <hive> [PATH]`           — direct children at PATH, also JSON
//!   * `show <hive> [PATH]`           — values of a key, also JSON
//!   * `find <hive> [filters]`        — pattern search across the key tree
//!
//! The library exposes the modules that back the [`rosregview` binary]
//! and the [`gen-completions`](src/bin/gen-completions.rs) helper.
//! External consumers rarely want this; it's here so the helper
//! binary can reuse the `Cli` derive types when emitting
//! shell-completion scripts and a man page without redeclaring them.

pub mod cli;
pub mod error;
pub mod hive;
pub mod output;
pub mod view;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Command};

/// CLI dispatcher shared by `src/main.rs` (the binary) and any
/// downstream callers that want to drive `rosregview` programmatically.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Info { path, format } => view::info::run(&path, format),
        Command::Tree {
            path,
            depth,
            format,
        } => view::tree::run(&path, depth, format),
        Command::List {
            path,
            key_path,
            format,
        } => view::list::run(&path, key_path.as_deref(), format),
        Command::Show {
            path,
            key_path,
            format,
        } => view::show::run(&path, key_path.as_deref(), format),
        Command::Find {
            path,
            name,
            value,
            case_sensitive,
            max_depth,
            format,
        } => view::find::run(
            &path,
            &name,
            value.as_deref(),
            case_sensitive,
            max_depth,
            format,
        ),
    }
}
