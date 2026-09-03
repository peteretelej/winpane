# Contributing to winpane

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- `clippy` and `rustfmt` components: `rustup component add clippy rustfmt`
- Windows 10 1903+ (builds target Win32 APIs)

## Documentation

- [Design overview](docs/design.md): architecture and key decisions, with deep dives under `docs/design/` (threading, rendering, surfaces, input, ffi, style)
- [JSON-RPC protocol](docs/protocol.md): method reference for `winpane-host`
- [Limitations](docs/limitations.md): known constraints and workarounds
- [Signing and distribution](docs/signing.md): code signing, SmartScreen, MSIX

Language guides for SDK users (Rust, Node.js, TypeScript, C, Go, Zig, Python) live under `docs/guides/` and on the docs site.

## Building

```sh
cargo build --workspace --all-targets
```

## Running tests

```sh
cargo test --workspace
```

## Pre-push hook

A pre-push script runs `cargo fmt --check`, `clippy`, and a workspace `cargo check` before each push; tests run in CI. Install it by symlinking:

**Bash (Linux, macOS, Git Bash on Windows):**

```sh
ln -sf ../../scripts/pre-push .git/hooks/pre-push
```

**PowerShell (Windows, requires admin or Developer Mode):**

```powershell
New-Item -ItemType SymbolicLink -Path .git\hooks\pre-push -Target ..\..\scripts\pre-push -Force
```

The same checks run in CI, so the hook keeps you from pushing code that will fail the pipeline.

## Linting

Workspace lints are configured in the root `Cargo.toml` under `[workspace.lints]`. All crates inherit them. To run the same checks locally:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Release Process

Releases are tag-driven: pushing a `v<NEW>` tag triggers the release workflow,
which builds binaries, creates the GitHub Release, and publishes to crates.io
and npm automatically.

1. Run the bump script (bumps all 8 versioned files in lockstep, then
   verifies with `cargo check` and `cargo fmt --check`):

   - PowerShell: `.\scripts\bump-version.ps1 -To <NEW>`
   - Bash: `./scripts/bump-version.sh <NEW>`

2. Spot-check `git diff` shows only version changes.
3. Make sure `cargo test --workspace` and clippy are green, then commit,
   tag `v<NEW>`, and push both.

Manual fallback (only if CI publishing fails): `cargo publish` in dependency
order (winpane-core, winpane, winpane-ffi, winpane-host), waiting ~30s
between crates for indexing.

AI agents: the `bump-version` skill wraps this same script; the `repo-guide`
skill covers everyday work in this repo.
