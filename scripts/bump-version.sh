#!/usr/bin/env bash
#
# Bumps all winpane package versions in lockstep.
#
# Usage:
#   ./scripts/bump-version.sh <new-version>            # auto-detects current version
#   ./scripts/bump-version.sh <old-version> <new-version>
#
# Examples:
#   ./scripts/bump-version.sh 0.1.2
#   ./scripts/bump-version.sh 0.1.1 0.1.2
set -euo pipefail

FILES=(
  crates/winpane-core/Cargo.toml
  crates/winpane/Cargo.toml
  crates/winpane-ffi/Cargo.toml
  crates/winpane-host/Cargo.toml
  bindings/node/Cargo.toml
  bindings/node/package.json
  bindings/node/npm/win32-x64-msvc/package.json
  bindings/node/npm/win32-arm64-msvc/package.json
)

if [[ $# -eq 1 ]]; then
  NEW="$1"
  OLD=$(grep -oP 'version\s*=\s*"\K[^"]+' crates/winpane-core/Cargo.toml | head -1)
  echo "Auto-detected current version: $OLD"
elif [[ $# -eq 2 ]]; then
  OLD="$1"
  NEW="$2"
else
  echo "Usage: $0 [old-version] <new-version>" >&2
  exit 1
fi

if [[ "$OLD" == "$NEW" ]]; then
  echo "Error: From and To versions are the same: $OLD" >&2
  exit 1
fi

echo "Bumping version: $OLD -> $NEW"
echo ""

ESCAPED_OLD=$(printf '%s\n' "$OLD" | sed 's/[.[\*^$/]/\\&/g')

for f in "${FILES[@]}"; do
  if [[ ! -f "$f" ]]; then
    echo "  WARNING: $f not found, skipping"
    continue
  fi
  count=$(grep -c "$OLD" "$f" || true)
  if [[ "$count" -eq 0 ]]; then
    echo "  WARNING: No occurrences of '$OLD' in $f"
    continue
  fi
  sed -i "s/$ESCAPED_OLD/$NEW/g" "$f"
  echo "  Updated $f ($count replacements)"
done

echo ""
echo "Running cargo check --workspace ..."
cargo check --workspace

echo ""
echo "Running cargo fmt --all -- --check ..."
cargo fmt --all -- --check || echo "WARNING: cargo fmt check failed — run 'cargo fmt --all' to fix."

echo ""
echo "Version bump complete: $OLD -> $NEW"
echo "Next steps:"
echo "  git add -A && git commit -m 'chore: bump version to $NEW'"
echo "  git push origin main"
echo "  git tag v$NEW && git push origin v$NEW"
