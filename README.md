# rosregview

Offline Windows registry hive viewer for ReactOS, written in Rust.

**Status:** early development (Phase 1 — cross-compile smoke test).

rosregview reads Windows `.hiv` / `.dat` files directly — no live registry
service required. It builds and runs on Windows, Linux, and macOS.

## Build

```bash
# Native (host platform)
cargo build --release

# Cross-compile to Windows (32-bit, i686)
cargo build --release --target i686-pc-windows-gnu
```

The cross-compile target needs a MinGW-w64 toolchain on `PATH`:

```bash
brew install mingw-w64  # macOS
```

## Usage (current MVP)

```bash
rosregview info <path-to.hiv>
```

Prints one-line summary: file size, parse status, root subkey count.

```text
$ ./target/release/rosregview info testdata/testhive
File:           testdata/testhive
Size:           159744 bytes
Parsed:         OK (nt-hive 0.3 accepted the file)
Root subkeys:   5
```

`tree`, `list`, `show`, and `find` are not implemented yet — see
`rosregview-plan.md` for the full roadmap.

## Cross-compiling to Windows

This MVP builds natively on macOS, Linux, and Windows from the same source
tree. Producing a Windows `.exe` from macOS requires a working MinGW-w64
toolchain — the Homebrew `mingw-w64@14` formula ships `libgcc_eh.a`
without the `_Unwind_Resume` symbol that the standard Rust link line
expects (a known issue with GCC ≥16 + mingw + Rust ≥1.94 combinations).

The supported workaround today is
[`cargo-zigbuild`](https://github.com/rust-cross/cargo-zigbuild):
Zig bundles its own MinGW stdlib that *does* ship unwinding, so it
sidesteps the broken `mingw-w64@14` `libgcc_eh` and produces a clean
PE32 i386 `.exe` from macOS:

```bash
brew install zig                     # one-time
cargo install cargo-zigbuild         # one-time
rustup target add i686-pc-windows-gnu
cargo zigbuild --release --target i686-pc-windows-gnu
# → target/i686-pc-windows-gnu/release/rosregview.exe (~650 KB)
```

For local Windows or Linux builds, no special toolchain is required —
just `cargo build --release`.

## Dependencies

- [`nt-hive`](https://crates.io/crates/nt-hive) (GPL-2.0-or-later) by Colin Finck
  — registry hive parser, the same crate used by ReactOS projects.

## License

GPL-2.0-or-later, matching ReactOS.
