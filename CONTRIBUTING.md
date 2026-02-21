# Contributing to winpane

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- `clippy` and `rustfmt` components: `rustup component add clippy rustfmt`
- Windows 10 1903+ (builds target Win32 APIs)

## Building

```sh
cargo build --workspace --all-targets
```

## Running tests

```sh
cargo test --workspace
```

## Pre-push hook

A pre-push script runs `cargo fmt --check`, `clippy`, and tests before each push. Install it by symlinking:

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
