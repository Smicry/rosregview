//! `rosregview` — entry point.
//!
//! The CLI surface lives in [`rosregview::cli`], per-subcommand
//! behaviour in [`rosregview::view`], shared hive/open/format helpers
//! in [`rosregview::hive`], and cross-subcommand output scaffolding in
//! [`rosregview::output`]. This binary is a one-liner that calls
//! [`rosregview::run`] so the same code path is reachable from tests
//! and from the helper `gen-completions` binary without duplication.

use anyhow::Result;

fn main() -> Result<()> {
    match rosregview::run() {
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::BrokenPipe) =>
        {
            Ok(())
        }
        result => result,
    }
}
