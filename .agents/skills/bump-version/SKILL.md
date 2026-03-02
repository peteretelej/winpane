---
name: bump-version
description: Bump all winpane package versions in lockstep and optionally tag a release
---

# Bump Version

Bump all winpane versions (Cargo.toml + package.json) in lockstep and verify the workspace builds.

## Usage

`/bump-version 0.1.2` — bump to a specific version (auto-detects current)
`/bump-version 0.1.1 0.1.2` — explicit old → new

## Files touched (8, all in lockstep)

| # | File | Field(s) |
|---|------|----------|
| 1 | `crates/winpane-core/Cargo.toml` | `version` |
| 2 | `crates/winpane/Cargo.toml` | `version`, `winpane-core` dep version |
| 3 | `crates/winpane-ffi/Cargo.toml` | `version`, `winpane` dep version |
| 4 | `crates/winpane-host/Cargo.toml` | `version`, `winpane` dep version |
| 5 | `bindings/node/Cargo.toml` | `version`, `winpane` dep version |
| 6 | `bindings/node/package.json` | `version` |
| 7 | `bindings/node/npm/win32-x64-msvc/package.json` | `version` |
| 8 | `bindings/node/npm/win32-arm64-msvc/package.json` | `version` |

## Procedure

### 1. Parse arguments

Extract `NEW_VERSION` from the user's input. If `OLD_VERSION` is not provided, auto-detect it from `crates/winpane-core/Cargo.toml` (the `version = "x.y.z"` field).

Validate that `NEW_VERSION` is a valid semver string and differs from `OLD_VERSION`.

### 2. Run the bump script

Use the project's bump script:

**PowerShell (Windows):**
```powershell
.\scripts\bump-version.ps1 -To <NEW_VERSION>
# or with explicit old version:
.\scripts\bump-version.ps1 -From <OLD_VERSION> -To <NEW_VERSION>
```

**Bash (Git Bash / CI):**
```bash
./scripts/bump-version.sh <NEW_VERSION>
# or with explicit old version:
./scripts/bump-version.sh <OLD_VERSION> <NEW_VERSION>
```

The script will:
1. Replace the old version with the new version in all 8 files
2. Run `cargo check --workspace` to verify dep resolution
3. Run `cargo fmt --all -- --check` to verify formatting
4. Print next steps (commit, tag, push)

### 3. Verify

If the script succeeds, spot-check that `git diff` shows ONLY version string changes in the expected 8 files. No other content should be modified.

### 4. Commit

```bash
git add -A
git commit -m "chore: bump version to <NEW_VERSION>"
```

### 5. (Optional) Tag and release

If the user wants to release immediately:

```bash
git push origin main
git tag v<NEW_VERSION>
git push origin v<NEW_VERSION>
```

Pushing the `v*` tag triggers `.github/workflows/release.yml` which automatically:
- Builds release binaries (DLL, EXE, MSI, native addon)
- Creates a GitHub Release with all assets
- Publishes to crates.io (`winpane-core` → `winpane` → `winpane-ffi` → `winpane-host`)
- Publishes to npm (`@winpane/win32-x64-msvc` + `winpane`)

### 6. Verify release (if tagged)

- GitHub Release page shows the tag with SDK zip, MSI, and .node assets
- crates.io: all 4 crates at the new version
- npm: `winpane` and `@winpane/win32-x64-msvc` at the new version

## Rollback

- **Delete tag:** `git tag -d v<VER> && git push origin :refs/tags/v<VER>`
- **Delete GitHub Release:** `gh release delete v<VER> --yes`
- **Yank crates.io** (permanent, only yank): `cargo yank --version <VER> -p <crate>`
- **Unpublish npm** (within 72h): `npm unpublish @winpane/win32-x64-msvc@<VER>`
