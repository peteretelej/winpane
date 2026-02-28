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

## Release Process

### Version Bumping

Update version in all these files (must match):

- `crates/winpane-core/Cargo.toml`
- `crates/winpane/Cargo.toml`
- `crates/winpane-ffi/Cargo.toml`
- `crates/winpane-host/Cargo.toml`
- `bindings/node/Cargo.toml`
- `bindings/node/package.json`
- `bindings/node/npm/win32-x64-msvc/package.json`
- `bindings/node/npm/win32-arm64-msvc/package.json`

### Creating a Release

1. Update version in all files listed above
2. Commit: `git commit -m "chore: bump version to X.Y.Z"`
3. Tag: `git tag vX.Y.Z`
4. Push: `git push origin main --tags`
5. Release workflow runs automatically, creating GitHub Release with artifacts
6. npm packages are published automatically via Trusted Publishing
7. Manual: publish crates to crates.io in dependency order:
   ```sh
   cargo publish -p winpane-core
   # wait ~30s for crates.io indexing
   cargo publish -p winpane
   cargo publish -p winpane-ffi
   cargo publish -p winpane-host
   ```

### Pre-release Checklist

- [ ] All tests pass (`cargo test --workspace`)
- [ ] Clippy passes (`cargo clippy --workspace --all-targets -- -D warnings`)
- [ ] Version numbers updated in all files
- [ ] CHANGELOG updated (if maintained)
