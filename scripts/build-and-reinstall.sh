#!/usr/bin/env bash
# Build the harness workspace and (re)install its binaries into ~/.cargo/bin.
#
# Installs two binaries by default:
#   harness      — the CLI (crate: harness-cli)
#   harness-gui  — the egui desktop front-end (crate: harness-gui)
#
# Usage:
#   scripts/build-and-reinstall.sh            # build + install both
#   scripts/build-and-reinstall.sh --cli-only # skip the GUI
#   scripts/build-and-reinstall.sh --gui-only # skip the CLI
#
# Idempotent: builds the whole workspace first (fast failure, warm cache), then
# `cargo install --force` rebuilds each crate in release mode and overwrites any
# existing binary on the cargo path.
set -euo pipefail

want_cli=1
want_gui=1
for arg in "$@"; do
  case "$arg" in
    --cli-only) want_gui=0 ;;
    --gui-only) want_cli=0 ;;
    -h|--help)
      sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "error: unknown option '$arg' (try --help)" >&2
      exit 2
      ;;
  esac
done

# Resolve the repo root from this script's location, so it works from anywhere.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HARNESS_DIR="$REPO_ROOT/harness"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found on PATH — install Rust (https://rustup.rs)" >&2
  exit 1
fi

if [ ! -f "$HARNESS_DIR/Cargo.toml" ]; then
  echo "error: no workspace Cargo.toml at $HARNESS_DIR" >&2
  exit 1
fi

cd "$HARNESS_DIR"

echo "==> Building workspace (release) ..."
cargo build --release --locked

if [ "$want_cli" -eq 1 ]; then
  echo
  echo "==> Installing harness (CLI) ..."
  cargo install --path "$HARNESS_DIR/crates/harness-cli" --locked --force
fi

if [ "$want_gui" -eq 1 ]; then
  echo
  echo "==> Installing harness-gui (desktop) ..."
  cargo install --path "$HARNESS_DIR/crates/harness-gui" --locked --force
fi

echo
echo "Done."
if [ "$want_cli" -eq 1 ] && command -v harness >/dev/null 2>&1; then
  echo "  $(command -v harness)  ->  $(harness --version)"
fi
if [ "$want_gui" -eq 1 ] && command -v harness-gui >/dev/null 2>&1; then
  echo "  $(command -v harness-gui)"
fi
