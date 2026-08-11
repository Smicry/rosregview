# Contributing to rosregview

Thanks for your interest in `rosregview`! This document covers the
contribution conventions used in this standalone repository. When (and
if) the code lands in the ReactOS main tree, the upstream
[`CONTRIBUTING.md`](https://github.com/reactos/reactos/blob/master/CONTRIBUTING.md)
takes precedence.

## Ground rules

1. **Real name + real email** in `git config user.name` /
   `user.email`. Anonymous / pseudonymous commits will be rejected at
   PR time.
2. **Legal affirmation**: every PR description must include the ReactOS
   legal affirmation:
   > *I hereby swear that I have not used nor seen the source code to
   > any version of the Windows operating system nor any Microsoft
   > product that may be related to the proposed project that is
   > under a license incompatible with contribution to `rosregview`,
   > including but not limited to the leaked Windows 2000 source code
   > and the Windows Research Kernel.*
3. **No Windows source references**. Search results, MS documentation,
   Win32 API *specifications* (the publicly documented ones) are fine;
   leaked source or WRK-derived knowledge is not.
4. **License = GPL-2.0-or-later**. New dependencies must be
   [compatible](DEPENDENCIES.md#compatibility-verdict). Anything
   non-permissive needs an explicit sign-off in the PR.

## Commit messages

Format:

```
[ROSREGVIEW] <imperative subject> (#<PR-NUM>)

<paragraph describing the change>

<footer line if needed>
```

Examples from history:

```
[ROSREGVIEW] Add `find` subcommand — Phase 2 final command
[ROSREGVIEW] Split monolithic main.rs into cli/error/hive/view/output modules
[ROSREGVIEW] Drop unused `thiserror` dependency
```

Subject rules:
- Imperative mood ("Add", "Fix", "Drop", "Bump"), no period at end.
- ≤ 72 characters.
- Backticks around code identifiers and file names.

Body rules:
- Wrap at ~72 columns.
- First line is the subject. Blank line. Then the body.
- Mention **why**, not **what** (the diff shows the what).
- Reference an issue / discussion if relevant.

## Branch & PR workflow

1. **Branch from `master`** with a topic prefix:
   - `feat/<short-topic>` — new subcommand or feature
   - `fix/<short-topic>` — bug fix
   - `refactor/<short-topic>` — no-behavior-change cleanups
   - `docs/<short-topic>` — documentation only
   - `ci/<short-topic>` — CI / workflow only

2. **One PR, one concern**. Don't bundle a refactor with a new feature.
   Reviewers will ask you to split it.

3. **CI must be green before review**:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --locked -- -D warnings
   cargo test --locked
   ```
   These three are now gated by `.github/workflows/ci.yml`.

4. **PR title = commit subject** (when squash-merging). The PR
   description can be longer; the subject must stand alone.

5. **Link your branch to an issue** if one exists; otherwise write a
   one-paragraph "why" in the PR body.

## Local pre-commit checks

There's no `.pre-commit-config.yaml` yet; until then run these by
hand:

```bash
# Format
cargo fmt --all

# Lint (deny warnings)
cargo clippy --all-targets --locked -- -D warnings

# Build (debug + release)
cargo build --locked
cargo build --release --locked

# All tests (unit + integration)
cargo test --locked

# Cross-compile smoke (if you have zig installed)
cargo zigbuild --release --target i686-pc-windows-gnu --locked
```

A clean run of all six is the prerequisite for opening a PR.

## Adding a new dependency

1. **Check the license**. See [DEPENDENCIES.md](DEPENDENCIES.md#compatibility-verdict).
2. **Pin minimally**. Prefer `^1.2` over `1`.
3. **Add it to `Cargo.toml`** under the right section (`[dependencies]`
   or `[dev-dependencies]`).
4. **Update `DEPENDENCIES.md`** with the new crate and its transitive
   dependents.
5. **Don't add a dependency for one function**. If it's < 30 lines and
   well-understood, vendor it inline and add a `// SPDX-License-Identifier:
   <original license>` comment.

## Adding a new subcommand

Currently the project has five (`info`, `tree`, `list`, `show`,
`find`). The skeleton for adding a sixth is:

1. Declare it in `src/cli.rs` (`Command` enum).
2. Create `src/view/<name>.rs` with:
   - `pub fn run(...) -> anyhow::Result<()>` entry point.
   - Private helpers.
   - `#[cfg(test)] mod tests` with at least:
     - one happy-path test against `testdata/testhive`,
     - one error-path test (missing key, malformed input, etc).
3. Wire the dispatch arm in `src/lib.rs` (`run()` function — the
   `src/main.rs` binary is just a one-line caller).
4. Update `README.md` "Usage" section.
5. Add at least one integration test in `tests/integration.rs`.

## Code style

- **Edition**: 2024 (see `Cargo.toml`).
- **Format**: `cargo fmt`. Don't fight it.
- **Lints**: `cargo clippy --all-targets -- -D warnings`. If a lint
  is genuinely wrong, `#[allow(...)]` with a comment explaining why.
- **Errors**: `anyhow::Result<T>` everywhere; `RosregError` is not
  introduced. `error::wrap_hive_error{, _owned}` is the single
  boundary for `nt_hive::NtHiveError` → `anyhow::Error`.
- **Module size**: aim for < 300 lines per file. If a file grows past
  that, look for a sub-module to split out.
- **Doc comments**: `///` on every public item, including test helpers
  used from outside the file.

## Testing strategy

| Layer | What it tests | Where |
|---|---|---|
| Unit | Per-module logic in isolation | `#[cfg(test)] mod tests` in every `src/**/*.rs` |
| Integration | End-to-end binary invocation against `testdata/testhive` | `tests/integration.rs` |
| Cross-compile | `i686-pc-windows-gnu` PE32 .exe is valid | `windows_exe_artifact_is_valid_pe32_when_present` (skipped if no .exe) |
| Real hives | Extract ReactOS LiveCD SYSTEM/SOFTWARE/SAM with `7z`, smoke-test against them | `.github/workflows/real-hives.yml` |

Adding a new unit test is cheaper than adding a new integration test
because unit tests run in milliseconds and don't require the binary
to be built first.

## Release process (when we get there)

`rosregview` is on `0.x` so every release is allowed to break
compatibility. After `1.0` we follow semver:

- **Patch** (0.0.x): bug fixes only.
- **Minor** (0.x.0): new subcommand or new flag.
- **Major** (x.0.0): breaking CLI changes (rare — the whole point of
  the CLI is stability).

## Communication

- **Issues**: GitHub Issues on this repo.
- **Discussions**: GitHub Discussions on this repo.
- **Upstream ReactOS**: when the PR lands in `reactos/reactos`,
  discussion moves to `ros-dev@reactos.org` and Mattermost.

## License

By contributing, you agree that your contributions will be licensed
under the same GPL-2.0-or-later terms as the rest of the project.
See [`LICENSE`](LICENSE).
