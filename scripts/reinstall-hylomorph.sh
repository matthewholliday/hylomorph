#!/usr/bin/env bash
# Build the Hylomorph CLI from source and (re)install it into ~/.cargo/bin.
#
# Usage: scripts/reinstall-hylomorph.sh
#
# Idempotent: `cargo install --force` rebuilds in release mode and overwrites
# any existing `hylomorph` binary on the cargo path.
set -euo pipefail

# Resolve the repo root from this script's location, so it works from anywhere.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HYLOMORPH_DIR="$REPO_ROOT/hylomorph"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found on PATH — install Rust (https://rustup.rs)" >&2
  exit 1
fi

if [ ! -f "$HYLOMORPH_DIR/Cargo.toml" ]; then
  echo "error: no Cargo.toml at $HYLOMORPH_DIR" >&2
  exit 1
fi

echo "Building and installing hylomorph from $HYLOMORPH_DIR ..."
cargo install --path "$HYLOMORPH_DIR/crates/hylomorph-cli" --locked --force

echo
echo "Installed: $(command -v hylomorph)"
hylomorph --version
