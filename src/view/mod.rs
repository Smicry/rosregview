//! Per-subcommand logic. Each submodule owns one top-level
//! subcommand and re-exports a single `run(...)` entry point.

pub mod find;
pub mod info;
pub mod list;
pub mod show;
pub mod tree;
