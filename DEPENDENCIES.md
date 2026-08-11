# Dependencies

This document tracks the license and provenance of every direct
dependency of `rosregview`, plus the transitive closure that's pulled
in at compile time. The audit is regenerated from `cargo tree`:

```bash
cargo tree --edges normal --format "{p} {l}" > deps.txt
```

The same data is encoded below for reviewers who don't have the Rust
toolchain handy.

## Why we care

`rosregview` is licensed under **GPL-2.0-or-later** because the
core dependency [`nt-hive`](#nt-hive-030) is GPL-2.0-or-later. GPL-2.0
is copyleft and propagates through derivative works, so we have to
make sure every dependency is *compatible* with GPL-2.0-or-later — i.e.
either GPL-2.0-or-later itself, or a permissive license that GPL-2.0
allows combining with.

The compatibility matrix below follows the [GNU license compatibility
list](https://www.gnu.org/licenses/license-list.en.html).

## Compatibility verdict

| License of dependency | GPL-2.0 compatible? |
|---|---|
| GPL-2.0-or-later (itself) | ✅ |
| MIT | ✅ |
| Apache-2.0 | ✅ (with "or later" of the GPL version we're under) |
| BSD-2-Clause / BSD-3-Clause | ✅ |
| Unlicense / 0BSD / Public Domain | ✅ |
| Unicode-3.0 | ⚠️ only for Unicode data; doesn't add copyleft |
| (no transitive license) | ❌ — should not exist |

**Verdict:** every dependency below is in the ✅ column. The
distribution license of the resulting binary is **GPL-2.0-or-later**.

## Direct dependencies (from `Cargo.toml`)

### `anyhow` 1.x — MIT OR Apache-2.0
- **Repository**: <https://github.com/dtolnay/anyhow>
- **Why we depend on it**: every fallible operation in the crate returns
  `anyhow::Result<T>`. Error contexts are attached via `.context(...)`.
- **License terms**: permissive dual MIT / Apache-2.0. Both are in the
  GNU compatibility list above. No copyleft, no patent clause beyond
  Apache-2.0's grant (which is forward-compatible with GPL-3 but not
  with GPL-2-only — we are GPL-2-or-later, which works).
- **Removed in**: `97e2686` (this crate previously used `thiserror` and
  was simplified to `anyhow`).

### `clap` 4.x — MIT OR Apache-2.0
- **Repository**: <https://github.com/clap-rs/clap>
- **Why we depend on it**: the CLI surface (`Cli`, `Command`, `OutputFormat`)
  is declared with `clap`'s derive macros.
- **License terms**: permissive dual MIT / Apache-2.0, same as `anyhow`.
- **Features used**: `derive` (for `#[derive(Parser)]` and friends).

### `nt-hive` 0.3 — GPL-2.0-or-later ⭐
- **Repository**: <https://github.com/ColinFinck/nt-hive>
- **Maintainer**: Colin Finck (ReactOS core developer, maintainer of
  `rosregview`'s core dependency).
- **Why we depend on it**: this is the actual hive parser. We deliberately
  do not re-implement registry parsing — `nt-hive` already does it and
  ships a `no_std`-friendly, well-tested implementation.
- **License terms**: GPL-2.0-or-later. This is why `rosregview` itself
  must be GPL-2.0-or-later.
- **Source**: `crates.io` (`nt-hive = "0.3"` in `Cargo.toml`).
- **Test fixture**: `testdata/testhive` (159 KB) is bundled from the
  `nt-hive` repository, also under GPL-2.0-or-later. See `testdata/README.txt`.

### `serde` 1.x — MIT OR Apache-2.0
- **Repository**: <https://github.com/serde-rs/serde>
- **Why we depend on it**: the `Stats` base type and every per-subcommand
  payload (`InfoPayload`, `TreeStats`, `ListStats`, `ShowStats`,
  `FindStats`) derive `Serialize` so JSON output is a one-liner.
- **License terms**: permissive dual MIT / Apache-2.0.
- **Features used**: `derive`.

### `serde_json` 1.x — MIT OR Apache-2.0
- **Repository**: <https://github.com/serde-rs/json>
- **Why we depend on it**: every `-f json` output sink ends in
  `serde_json::to_string_pretty(...)` and is parsed by integration tests
  via `serde_json::from_slice`.
- **License terms**: permissive dual MIT / Apache-2.0.

### `clap_complete` 4.x — MIT OR Apache-2.0
- **Repository**: <https://github.com/clap-rs/clap>
- **Why we depend on it**: the `gen-completions` helper binary uses
  `clap_complete::generate_to` to emit shell-completion scripts for
  bash, zsh, fish, and PowerShell from the same `clap` derive types
  the `rosregview` binary uses.
- **License terms**: permissive dual MIT / Apache-2.0.
- **Used only at**: `cargo run --bin gen-completions -- --outdir <DIR>`,
  and in the `completions / drift check` CI job.

### `clap_mangen` 0.3.x — MIT OR Apache-2.0
- **Repository**: <https://github.com/clap-rs/clap>
- **Why we depend on it**: the `gen-completions` helper binary emits
  the `rosregview(1)` man page via `clap_mangen::Man`.
- **License terms**: permissive dual MIT / Apache-2.0.
- **Transitive dep**: [`roff`](https://crates.io/crates/roff) 1.x (MIT).

## Transitive dependencies

Every transitive crate pulled in by the seven direct deps above:

| Crate | Version | License |
|---|---|---|
| `anstream` | 1.0.0 | MIT OR Apache-2.0 |
| `anstyle` | 1.0.14 | MIT OR Apache-2.0 |
| `anstyle-parse` | 1.0.0 | MIT OR Apache-2.0 |
| `anstyle-query` | 1.1.5 | MIT OR Apache-2.0 |
| `anstyle-wincon` | 3.0.11 | MIT OR Apache-2.0 |
| `autocfg` | 1.5.1 | MIT OR Apache-2.0 |
| `bitflags` | 2.13.1 | MIT OR Apache-2.0 |
| `clap_builder` | 4.6.2 | MIT OR Apache-2.0 |
| `clap_derive` | 4.6.3 (proc-macro) | MIT OR Apache-2.0 |
| `clap_lex` | 1.1.0 | MIT OR Apache-2.0 |
| `colorchoice` | 1.0.5 | MIT OR Apache-2.0 |
| `enumn` | 0.1.14 (proc-macro) | MIT OR Apache-2.0 |
| `heck` | 0.5.0 | MIT OR Apache-2.0 |
| `is_terminal_polyfill` | 1.70.2 | MIT OR Apache-2.0 |
| `itoa` | 1.0.18 | MIT OR Apache-2.0 |
| `memchr` | 2.8.3 | Unlicense OR MIT |
| `memoffset` | 0.9.1 | MIT |
| `once_cell_polyfill` | 1.70.2 | MIT OR Apache-2.0 |
| `proc-macro2` | 1.0.107 | MIT OR Apache-2.0 |
| `quote` | 1.0.47 | MIT OR Apache-2.0 |
| `serde_core` | 1.0.229 | MIT OR Apache-2.0 |
| `serde_derive` | 1.0.229 (proc-macro) | MIT OR Apache-2.0 |
| `strsim` | 0.11.1 | MIT |
| `syn` | 2.0.119 | MIT OR Apache-2.0 |
| `syn` | 3.0.2 | MIT OR Apache-2.0 |
| `thiserror` | 2.0.19 | MIT OR Apache-2.0 |
| `thiserror-impl` | 2.0.19 (proc-macro) | MIT OR Apache-2.0 |
| `unicode-ident` | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| `utf8parse` | 0.2.2 | Apache-2.0 OR MIT |
| `windows-link` | 0.2.1 | MIT OR Apache-2.0 |
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 |
| `zerocopy` | 0.8.54 | BSD-2-Clause OR Apache-2.0 OR MIT |
| `zerocopy-derive` | 0.8.54 (proc-macro) | BSD-2-Clause OR Apache-2.0 OR MIT |
| `zmij` | 1.0.23 | MIT |
| `roff` | 1.1.1 | MIT |

(The two `syn` versions are both pulled in transitively — `serde_derive`
and `thiserror-impl` require `syn` 2.x; `clap_derive` does too. Some
intermediate path appears to want 3.x; in practice only one major
version is actually compiled in any single binary.)

## Build-time tools (not in the binary)

These are not linked into the final `rosregview` binary; they're
build-time only and don't affect the distribution license.

| Tool | License | Why |
|---|---|---|
| `rustc` | MIT OR Apache-2.0 | Compiler |
| `cargo` | MIT OR Apache-2.0 | Build tool |
| `cargo-zigbuild` | MIT OR Apache-2.0 | Cross-compile via Zig (used in CI) |
| `Zig` | MIT | C/C++ cross-compiler standard library for MinGW (used in CI) |
| `GitHub Actions` runners | Various (proprietary service) | CI |

## Test-only crates

Listed in `[dev-dependencies]` in `Cargo.toml`. Not currently used
beyond what `cargo test` already pulls in:

| Crate | Version | License | Use |
|---|---|---|---|
| (none yet) | — | — | All tests currently run against the test binary directly. |

## How to regenerate this document

```bash
cargo tree --edges normal --format "{p} {l}"
```

If a future dependency adds a non-permissive license, this command
will surface it; we then need to either drop the dependency or
re-license the binary accordingly.

## References

- GNU license list: <https://www.gnu.org/licenses/license-list.en.html>
- `nt-hive` design notes: <https://colinfinck.de/categories/rust/>
- ReactOS `CONTRIBUTING.md` (when the PR lands upstream): see the
  reactos/reactos repository root.
