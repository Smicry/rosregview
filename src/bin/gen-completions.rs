//! `gen-completions` — emit shell-completion scripts and a man page
//! for `rosregview`. Built only when the `completions` Cargo feature
//! is enabled:
//!
//! ```bash
//! cargo run --features completions --bin gen-completions -- \
//!     --outdir completions/
//! ```
//!
//! Output files (relative to `--outdir`):
//!
//!   completions/rosregview.bash
//!   completions/_rosregview        (zsh)
//!   completions/rosregview.fish
//!   completions/_rosregview.ps1    (PowerShell)
//!   man/rosregview.1
//!
//! `completions/` and `man/` are subdirectories under `--outdir`; the
//! helper creates them if missing. The script exits 0 on success and
//! a non-zero code on any I/O or rendering failure.

use std::path::{Path, PathBuf};

use clap::CommandFactory;
use clap_complete::{Shell, generate_to};
use clap_mangen::Man;

use rosregview::cli::Cli;

fn main() -> anyhow::Result<()> {
    let parent = parse_args()?;

    // Layout:
    //   <parent>/completions/<shell-specific files>
    //   <parent>/man/rosregview.1
    let completions_dir = parent.join("completions");
    let man_dir = parent.join("man");
    std::fs::create_dir_all(&completions_dir)?;
    std::fs::create_dir_all(&man_dir)?;

    let mut cmd = Cli::command();
    let bin_name = "rosregview";

    println!("Writing completions to {}", completions_dir.display());
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell] {
        let path = generate_to(shell, &mut cmd, bin_name, &completions_dir)?;
        println!("  wrote {}", path.display());
    }

    let man = Man::new(cmd);
    let man_path = man_dir.join(format!("{bin_name}.1"));
    let mut file = std::fs::File::create(&man_path)?;
    man.render(&mut file)?;
    println!("  wrote {}", man_path.display());

    Ok(())
}

fn parse_args() -> anyhow::Result<PathBuf> {
    // Use a minimal arg parser instead of pulling in clap for the
    // generator itself — we want the helper's CLI to be tiny and
    // dependency-free.
    let mut parent: Option<PathBuf> = None;
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "--outdir" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--outdir requires a path argument"))?;
                parent = Some(PathBuf::from(value));
            }
            other => {
                anyhow::bail!("unknown argument: {other}");
            }
        }
    }
    Ok(parent.unwrap_or_else(|| Path::new(".").to_path_buf()))
}

fn print_help() {
    println!(
        "gen-completions — emit shell completions and a man page for rosregview

USAGE:
    gen-completions [--outdir <DIR>]

ARGS:
    --outdir <DIR>    Parent directory under which to create
                      completions/ and man/ subdirectories.
                      Defaults to the current working directory.

OUTPUT:
    <outdir>/completions/rosregview.bash
    <outdir>/completions/_rosregview          (zsh)
    <outdir>/completions/rosregview.fish
    <outdir>/completions/_rosregview.ps1      (PowerShell)
    <outdir>/man/rosregview.1"
    );
}
