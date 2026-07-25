# rosregview

[![CI](https://github.com/Smicry/rosregview/actions/workflows/ci.yml/badge.svg)](https://github.com/Smicry/rosregview/actions/workflows/ci.yml)
[![License: GPL-2.0-or-later](https://img.shields.io/badge/License-GPL--2.0--or--later-blue.svg)](LICENSE)
[![Rust 1.94+](https://img.shields.io/badge/rust-1.94%2B-orange.svg)](https://www.rust-lang.org)

Offline Windows registry hive viewer for ReactOS, written in Rust.

Reads Windows `.hiv` / `.dat` files directly — no live registry
service required. Cross-platform: builds and runs on Linux, macOS,
and Windows. GPL-2.0-or-later to match ReactOS.

## Status

**Phase 2 complete.** All planned subcommands implemented
(`info`, `tree`, `list`, `show`, `find`) with shared `-f json`
output. Ready for review against the ReactOS tree (collab) — letter
to the maintainer sent, awaiting reply.

## Subcommands

### `info <hive> [-f json]` — overview

Single-line summary: file size, root subkey count, hive minor version.

```text
$ ./target/release/rosregview info testdata/testhive
File:           testdata/testhive
Size:           159744 bytes
Parsed:         OK (nt-hive 0.3, minor version 5)
Root subkeys:   5
```

The `minor version` number comes from the hive base block and maps to a
known Windows release:

| minor_version | Windows release |
|---|---|
| 0 | NT 3.1 Beta |
| 1 | NT 3.1 |
| 2 | NT 3.5 |
| 3 | NT 4.0 |
| 4 | XP Beta |
| 5 | XP |
| 6 | Vista / Server 2008+ |

(Values outside this table are accepted by nt-hive but not assigned a
friendly name; the raw integer is still emitted.)

### `tree <hive> [--depth N] [-f json]` — recursive key hierarchy

Indented tree of all keys under the root. ASCII-only indent, UTF-8
names preserved (including supplementary-plane characters stored as
UTF-16 surrogate pairs in the hive).

```text
$ ./target/release/rosregview tree testdata/testhive --depth 1
File:     testdata/testhive
Size:     159744 bytes
Depth:    0..=1

<root>
  big-data-test
  character-encoding-test
  data-test
  subkey-test
  subpath-test
```

### `list <hive> [KEY_PATH] [-f json]` — direct children with counts

Like the Unix `ls`. Per child: name, subkey count, value count.

```text
$ ./target/release/rosregview list testdata/testhive data-test
File:    testdata/testhive
At:      data-test

Name                              Subkeys    Values
──────────────────────────────────────────────────────
reg-sz                                   0         1
reg-sz-with-terminating-nul              0         1
reg-expand-sz                            0         1
reg-multi-sz                             0         1
reg-multi-sz-big                         0         1
dword                                    0         1
dword-big-endian                         0         1
qword                                    0         1
binary                                   0         1

Total: 9 values
```

(Yes — those `Subkeys: 0` numbers are a presentation quirk of `list`:
"how many subkeys does each child have". The 9 *values of the
parent key* are what `show data-test` reports below.)

### `show <hive> [KEY_PATH] [-f json]` — values of a key

Decodes every value of a key according to its on-disk REG_* type:

| Type | Human output | JSON |
|---|---|---|
| REG_SZ / REG_EXPAND_SZ | decoded UTF-16 string (trailing NUL stripped) | JSON string |
| REG_MULTI_SZ | UTF-16 lines, joined with ` \| ` | JSON string array |
| REG_DWORD / REG_DWORD_BIG_ENDIAN | decimal + hex | JSON number (u32) |
| REG_QWORD | decimal + hex | JSON number (u64) |
| REG_BINARY | hex dump (32 bytes preview) | JSON number array |
| REG_LINK / REG_RESOURCE_* / REG_NONE / unknown | raw hex | JSON number array |
| anything nt-hive can't decode | `<decode error>` | JSON string error marker |

```text
$ ./target/release/rosregview show testdata/testhive data-test
File:    testdata/testhive
At:      data-test

Name                              Type                      Data
────────────────────────────────────────────────────────────────────────
reg-sz                            REG_SZ                    sz-test
reg-sz-with-terminating-nul        REG_SZ                    sz-test
reg-expand-sz                     REG_EXPAND_SZ             sz-test
reg-multi-sz                      REG_MULTI_SZ              multi-sz-test  |  line2
reg-multi-sz-big                  REG_MULTI_SZ              0123456789012345678901…
dword                             REG_DWORD                 42 (0x0000002a)
dword-big-endian                  REG_DWORD_BIG_ENDIAN      704643072 (0x2a000000)
qword                             REG_QWORD                 18446744073709551615 (0x…
binary                            REG_BINARY                01 02 03 04 05

Total: 9 values
```

### `find <hive> [-n NAME]… [-v VALUE] [--case-sensitive] [--max-depth N] [-f json]` — pattern search

Recursively walks the hive, collecting any key whose name matches
`-n` (substring, repeatable for any-of, case-insensitive by default) and
any value whose name+decoded-data contains `-v`. With no filters this
just enumerates paths up to `--max-depth`.

Value matching reuses `show`'s type-aware decoding so REG_SZ is
substring-matched against the UTF-8 string, REG_DWORD/QWORD against
the decimal+hex rendering, REG_BINARY against the hex-dump string,
etc.

```text
$ rosregview find testdata/testhive -n test --max-depth 1
File:     testdata/testhive
Patterns: name~=["test"]  value~=None  case_sensitive=false
Max depth: 1
Scanned 6 keys, matched 5 key(s).

big-data-test
character-encoding-test
data-test
subkey-test
subpath-test
```

```text
$ rosregview find testdata/testhive -v 42 --max-depth 2
…
data-test
    • dword: REG_DWORD = 42 (0x0000002a)
```

## Build

### Native (Linux / macOS / Windows hosts)

```bash
cargo build --release
./target/release/rosregview info testdata/testhive
```

### Cross-compile to Windows from Linux or Windows hosts

```bash
rustup target add i686-pc-windows-gnu
cargo build --release --target i686-pc-windows-gnu
# requires MinGW-w64 in PATH (Linux: apt install mingw-w64; macOS: brew install mingw-w64)
```

### Cross-compile to Windows from macOS via `cargo-zigbuild`

The Homebrew `mingw-w64@14` formula ships `libgcc_eh.a` without the
`_Unwind_Resume` symbol that the standard Rust link line expects,
so plain `cargo build --target i686-pc-windows-gnu` fails on macOS
(a known issue with GCC ≥16 + mingw + Rust ≥1.94 combinations).

The supported macOS cross-compile path uses
[`cargo-zigbuild`](https://github.com/rust-cross/cargo-zigbuild);
Zig bundles its own MinGW stdlib with full unwinding support:

```bash
brew install zig                     # one-time, ~150 MB
cargo install cargo-zigbuild         # one-time, ~1-2 min
rustup target add i686-pc-windows-gnu
cargo zigbuild --release --target i686-pc-windows-gnu
# → target/i686-pc-windows-gnu/release/rosregview.exe (~650 KB PE32 i386)
```

## Tests

```bash
cargo test
```

56 tests in total: 20 end-to-end integration tests against the
bundled `testdata/testhive` fixture (from the `nt-hive` project),
plus 36 in-module unit tests across `cli`, `error`, `hive::*`,
`view::*`, and `output::*`. Coverage spans:

- Happy-path parsing on the bundled hive fixture.
- Three error paths (missing file / non-hive / unknown output format).
- JSON shape for every subcommand.
- `--depth N` correctness, multi-level `KEY_PATH` descent.
- UTF-16 surrogate names round-tripping.
- All nine tested REG_* types.
- `find` name+value substring matching (case-insensitive default,
  plus hex/decimal data previews).
- A `PE32` header check on the cross-compiled `.exe` that skips
  cleanly when the artifact is absent.

CI runs natively on Linux, macOS, and Windows, plus the
`cargo-zigbuild` cross compile.

## CI test tiers

Three independent workflows run in GitHub Actions:

| Workflow | What it covers | Trigger |
|---|---|---|
| `ci.yml` | Native build + integration tests on Linux/macOS/Windows, `i686-pc-windows-gnu` cross-compile via Zig, `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and a `completions / drift check` job that fails if the checked-in `completions/` and `man/` files drift from what `gen-completions` would emit today. | every push, every PR |
| `real-hives.yml` | Downloads the latest ReactOS LiveCD (a `.7z` from `iso.reactos.org/livecd/`), extracts the actual `SYSTEM`/`SOFTWARE`/`SAM`/`SECURITY`/`DEFAULT` hive files, and smoke-tests `rosregview` against them. No QEMU/KVM needed. | PRs touching source, weekly cron, manual |

The `real-hives` workflow keeps us honest against real-world hive data
that the bundled 159 KB `testdata/testhive` fixture can't cover —
multi-MB hives, different minor-version variants, hive files that
ship with the current ReactOS nightly.

## Shell completions and man page

Shell-completion scripts (bash / zsh / fish / PowerShell) and a
man page live in `completions/` and `man/` respectively. They are
generated by the in-tree helper binary `gen-completions`:

```bash
cargo run --bin gen-completions -- --outdir .
# writes completions/rosregview.bash, _rosregview (zsh),
#        rosregview.fish, _rosregview.ps1 (PowerShell), and
#        man/rosregview.1
```

Install locations (typical):
- **bash**: source `completions/rosregview.bash` from your `.bashrc`.
- **zsh**: copy `completions/_rosregview` into a directory in
  `$fpath` (e.g. `/usr/local/share/zsh/site-functions/`).
- **fish**: copy `completions/rosregview.fish` into
  `~/.config/fish/completions/`.
- **man**: copy `man/rosregview.1` into `~/.local/share/man/man1/`
  (and run `mandb`).

The CI `completions / drift check` job regenerates these files and
fails the build if the checked-in copies don't match — i.e. any PR
that changes the CLI surface must run the generator and commit the
result.

## Dependencies

- [`nt-hive`](https://crates.io/crates/nt-hive) 0.3 (GPL-2.0-or-later) —
  Colin Finck's registry hive parser, the same crate ReactOS projects
  consume; the only Rust dependency that touches hive bytes.
- `clap` 4.x with `derive` — CLI parsing and `--help`/`--version`
  rendering.
- `serde` + `serde_json` 1.x — JSON output path behind `-f json`.
- `anyhow` 1.x — `Result` propagation and rich error contexts.

`Cargo.lock` pins exact versions for reproducibility.

## Limitations

`rosregview` is intentionally a thin read-only viewer. Things it does
**not** do:

- **Hive writes / edits.** No `set`, `delete`, or `import`. Use
  `regedit` on a live system, or `offreg.dll` in Windows, for that.
- **Transaction log replay.** `nt-hive` 0.3 reads the primary hive
  only. The `.LOG` and `.LOG1`/`.LOG2` files are not consulted.
  Corrupted-but-recoverable hives (e.g. after a power loss) may not
  parse. See
  [nt-hive#issues](https://github.com/ColinFinck/nt-hive/issues) for
  the current recovery coverage.
- **Registry value decoding** is best-effort. `REG_BINARY`, `REG_NONE`,
  `REG_LINK`, `REG_RESOURCE_LIST`, `REG_FULL_RESOURCE_DESCRIPTOR`,
  `REG_RESOURCE_REQUIREMENTS_LIST`, and unknown future codes render
  as a hex dump. Strings are decoded as UTF-16-LE lossy (no
  round-trip preservation).
- **Registry path navigation** is case-insensitive at the byte level
  (Latin-1 only). Unicode case folding beyond ASCII is approximate.
- **No live registry API.** We only read hive *files*. To inspect a
  running Windows or ReactOS installation, copy the hive files out
  first (`%SystemRoot%\System32\config`).
- **i686-pc-windows-gnu only.** We cross-compile to 32-bit Windows
  via Zig. There is no `x86_64-pc-windows-gnu`, no MSVC target, no
  ARM. Tracked separately; add a target as needed.

If any of these is a blocker for your use case, file an issue with
the specific scenario (hive path + subcommand + expected output).

## License

GPL-2.0-or-later. See `LICENSE` for the full text. The full GPL-2.0
verbatim is copied from `reactos/COPYING` to keep the license file
byte-identical with ReactOS's.
