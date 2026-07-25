//! rosregview — Offline Windows registry hive viewer for ReactOS.
//!
//! See `Cargo.toml` for project metadata, and `rosregview-plan.md` in
//! the repository root for the wider plan and design notes.
//!
//! Subcommands currently implemented:
//!   * `info <hive>`                  — overview + JSON via `-f json`
//!   * `tree <hive> [--depth N]`      — recursive key tree, also JSON
//!   * `list <hive> [PATH]`           — direct children at PATH, also JSON
//!   * `show <hive> [PATH]`           — values of a key, also JSON
//!   * `find <hive> [filters]`        — pattern search across the key tree
//!
//! This binary is a thin dispatcher. The CLI surface lives in [`cli`],
//! per-subcommand behaviour in [`view`], shared hive/open/format helpers
//! in [`hive`], and cross-subcommand output scaffolding in
//! [`output`].

use anyhow::Result;
use clap::Parser;

mod cli;
mod error;
mod hive;
mod output;
mod view;

use cli::{Cli, Command};

fn main() -> Result<()> {
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
