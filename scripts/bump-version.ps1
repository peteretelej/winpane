<#
.SYNOPSIS
    Bumps all winpane package versions in lockstep.
.DESCRIPTION
    Replaces the old version string with a new version string across all 8
    versioned files (Cargo.toml + package.json). Then runs cargo check to
    verify workspace dep resolution.
.PARAMETER From
    The current version to replace (e.g. "0.1.1").
    If omitted, auto-detected from crates/winpane-core/Cargo.toml.
.PARAMETER To
    The new version to set (e.g. "0.1.2"). Required.
.EXAMPLE
    .\scripts\bump-version.ps1 -To 0.1.2
.EXAMPLE
    .\scripts\bump-version.ps1 -From 0.1.1 -To 0.1.2
#>
param(
    [string]$From,
    [Parameter(Mandatory)][string]$To
)

$ErrorActionPreference = 'Stop'

$files = @(
    'crates/winpane-core/Cargo.toml',
    'crates/winpane/Cargo.toml',
    'crates/winpane-ffi/Cargo.toml',
    'crates/winpane-host/Cargo.toml',
    'bindings/node/Cargo.toml',
    'bindings/node/package.json',
    'bindings/node/npm/win32-x64-msvc/package.json',
    'bindings/node/npm/win32-arm64-msvc/package.json'
)

# Auto-detect current version if -From not provided
if (-not $From) {
    $coreCargo = Get-Content 'crates/winpane-core/Cargo.toml' -Raw
    if ($coreCargo -match 'version\s*=\s*"([^"]+)"') {
        $From = $Matches[1]
        Write-Host "Auto-detected current version: $From" -ForegroundColor Cyan
    } else {
        Write-Error "Could not detect current version from crates/winpane-core/Cargo.toml. Use -From explicitly."
        exit 1
    }
}

if ($From -eq $To) {
    Write-Error "From and To versions are the same: $From"
    exit 1
}

Write-Host "Bumping version: $From -> $To" -ForegroundColor Green
Write-Host ""

$escapedFrom = [regex]::Escape($From)

foreach ($f in $files) {
    if (-not (Test-Path $f)) {
        Write-Warning "File not found, skipping: $f"
        continue
    }
    $content = Get-Content $f -Raw
    $count = ([regex]::Matches($content, $escapedFrom)).Count
    if ($count -eq 0) {
        Write-Warning "No occurrences of '$From' in $f"
        continue
    }
    $newContent = $content -replace $escapedFrom, $To
    Set-Content $f $newContent -NoNewline
    Write-Host "  Updated $f ($count replacement$(if($count -ne 1){'s'}))" -ForegroundColor Gray
}

Write-Host ""
Write-Host "Running cargo check --workspace ..." -ForegroundColor Cyan
cargo check --workspace
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo check failed! Review the version changes."
    exit 1
}

Write-Host ""
Write-Host "Running cargo fmt --all -- --check ..." -ForegroundColor Cyan
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) {
    Write-Warning "cargo fmt check failed — run 'cargo fmt --all' to fix."
}

Write-Host ""
Write-Host "Version bump complete: $From -> $To" -ForegroundColor Green
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "  git add -A && git commit -m 'chore: bump version to $To'"
Write-Host "  git push origin main"
Write-Host "  git tag v$To && git push origin v$To"
