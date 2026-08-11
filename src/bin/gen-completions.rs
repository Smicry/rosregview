//! `gen-completions` — emit shell-completion scripts and a man page
//! for `rosregview`. Always compiled; pull in the `clap_complete` and
//! `clap_mangen` deps unconditionally (the savings of a feature flag
//! would not justify the conditional-compile complexity at this scale).
//!
//! ```bash
//! cargo run --bin gen-completions -- --outdir .
//! ```
//!
//! Output files (relative to `--outdir`):
//!
//!   completions/rosregview.bash
//!   completions/_rosregview        (zsh)
//!   completions/rosregview.fish
//!   completions/_rosregview.ps1    (PowerShell)
//!   man/rosregview*.1
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

    let subcommands: Vec<_> = cmd.get_subcommands().cloned().collect();
    let man_path = man_dir.join(format!("{bin_name}.1"));
    render_man(cmd, &man_path)?;
    println!("  wrote {}", man_path.display());
    for subcommand in subcommands {
        let page_name = format!("{bin_name}-{}", subcommand.get_name());
        let path = man_dir.join(format!("{page_name}.1"));
        // clap stores command names as static strings unless its optional
        // `string` feature is enabled. This short-lived generator owns only
        // one tiny allocation per subcommand, so promoting them is harmless.
        let command_name: &'static str = Box::leak(page_name.into_boxed_str());
        render_man(subcommand.name(command_name), &path)?;
        println!("  wrote {}", path.display());
    }

    Ok(())
}

fn render_man(command: clap::Command, path: &Path) -> anyhow::Result<()> {
    let mut rendered = Vec::new();
    Man::new(command).render(&mut rendered)?;
    let rendered = String::from_utf8(rendered)?;
    let normalized = rendered
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(path, format!("{normalized}\n"))?;
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
    <outdir>/man/rosregview*.1"
    );
}
