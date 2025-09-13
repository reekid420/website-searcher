#!/bin/sh

set -e

info()  { printf "[info] %s\n" "$1"; }
warn()  { printf "[warn] %s\n" "$1"; }
err()   { printf "[err ] %s\n" "$1"; }

command_exists() { command -v "$1" >/dev/null 2>&1; }

install_node() {
  if command_exists node; then return; fi
  info "Installing Node.js (using package manager if available)"
  if command_exists brew; then
    brew install node
  elif command_exists apt-get; then
    sudo apt-get update -y && sudo apt-get install -y curl ca-certificates gnupg
    curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
    sudo apt-get install -y nodejs
  elif command_exists dnf; then
    sudo dnf module enable -y nodejs:20 || true
    sudo dnf install -y nodejs
  elif command_exists pacman; then
    sudo pacman -Sy --noconfirm nodejs npm
  else
    warn "Please install Node.js LTS from https://nodejs.org and re-run."
  fi
}

activate_pnpm_via_corepack() {
  if ! command_exists node; then return; fi
  if command_exists pnpm; then return; fi
  info "Activating pnpm via Corepack"
  corepack enable || true
  corepack prepare pnpm@latest --activate || true
}

install_rustup() {
  if command_exists cargo; then return; fi
  info "Installing Rust (rustup)"
  if command_exists brew; then
    brew install rustup-init
    rustup-init -y
  elif command_exists apt-get; then
    curl https://sh.rustup.rs -sSf | sh -s -- -y
  elif command_exists dnf; then
    curl https://sh.rustup.rs -sSf | sh -s -- -y
  elif command_exists pacman; then
    curl https://sh.rustup.rs -sSf | sh -s -- -y
  else
    curl https://sh.rustup.rs -sSf | sh -s -- -y
  fi
  . "$HOME/.cargo/env"
}

ensure_rust_components() {
  if ! command_exists cargo; then return; fi
  info "Ensuring rustfmt and clippy components"
  rustup component add rustfmt || true
  rustup component add clippy || true
}

ensure_cargo_tool() {
  NAME="$1"; CHECK_CMD="$2"; CRATE="$3"; ARGS="$4"
  if sh -c "$CHECK_CMD" >/dev/null 2>&1; then return; fi
  if ! command_exists cargo; then
    warn "$NAME requires Cargo; skipping (Cargo not found)"
    return
  fi
  info "Installing $NAME ($CRATE)"
  if [ -n "$ARGS" ]; then
    cargo install "$CRATE" $ARGS || true
  else
    cargo install "$CRATE" || true
  fi
}

printf "==> Quickstart: installing prerequisites\n"

install_node
activate_pnpm_via_corepack

install_rustup
# shellcheck disable=SC1090
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
ensure_rust_components

# Cargo tools used by scripts/build/tests
ensure_cargo_tool "Tauri CLI"      "cargo tauri --version"   "tauri-cli" "--locked"
ensure_cargo_tool "cargo-audit"    "cargo audit -V"          "cargo-audit" ""
ensure_cargo_tool "cargo-nextest"  "cargo nextest --version" "cargo-nextest" ""
ensure_cargo_tool "cargo-llvm-cov" "cargo-llvm-cov --version" "cargo-llvm-cov" ""

printf "\n==> Versions\n"
command_exists node  && node -v  | sed 's/^/node    : /'
command_exists pnpm  && pnpm -v  | sed 's/^/pnpm    : /'
command_exists cargo && cargo -V  | sed 's/^/cargo   : /'
command_exists rustc && rustc -V  | sed 's/^/rustc   : /'
(cargo tauri --version 2>/dev/null || true) | sed 's/^/tauri   : /'
(cargo audit -V 2>/dev/null        || true) | sed 's/^/audit   : /'
(cargo nextest --version 2>/dev/null || true) | sed 's/^/nextest : /'
(cargo-llvm-cov --version 2>/dev/null || true) | sed 's/^/llvm-cov: /'

printf "\n[info] Quickstart complete. You can now run ./compile.sh (Linux/macOS) or ./compile.ps1 (Windows).\n"


