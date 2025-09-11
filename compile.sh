#!/usr/bin/env bash
set -euo pipefail

# Format
if cargo fmt --all -- --check; then
  echo "Formatting is correct"
else
  echo "Formatting is incorrect, fixing..."
  cargo fmt --all
fi

# Lint
cargo clippy --all-targets -- -D warnings

# Build (debug)
cargo build --workspace

# Test
cargo test --locked --workspace

# Build (release)
cargo build --workspace --release

exit 0