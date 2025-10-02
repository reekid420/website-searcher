#!/bin/bash
set -euo pipefail

# Flags and logging
VERBOSE=0
LOG_ENABLED=0
LOG_FILE=""
WANT_DEB=0
WANT_RPM=0

# Parse optional flags early so they affect all steps
while [ "$#" -gt 0 ]; do
  case "$1" in
    -d|--deb) WANT_DEB=1 ;;
    -r|--rpm) WANT_RPM=1 ;;
    -v|--verbose) VERBOSE=1 ;;
    -l|--log)
      LOG_ENABLED=1
      LOG_FILE="compile-script-$(date +%Y%m%d-%H%M%S).log"
      ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
  shift
done

if [ "$LOG_ENABLED" -eq 1 ]; then
  echo "Logging to: $LOG_FILE"
fi

# Run a command with verbosity/logging controls
run_cmd() {
  if [ "$VERBOSE" -eq 1 ] && [ "$LOG_ENABLED" -eq 1 ]; then
    eval "$*" 2>&1 | tee -a "$LOG_FILE"
  elif [ "$VERBOSE" -eq 1 ]; then
    eval "$*"
  elif [ "$LOG_ENABLED" -eq 1 ]; then
    bash -c "$*" 1>>"$LOG_FILE" 2> >(tee -a "$LOG_FILE" >&2)
  else
    # Quiet stdout but preserve stderr on console for visibility
    bash -c "$*" 1>/dev/null
  fi
}

# Cargo quiet flag unless verbose
if [ "$VERBOSE" -eq 1 ]; then
  CARGO_Q=""
else
  CARGO_Q="-q"
fi

# Format
if cargo fmt --all -- --check; then
  echo "Formatting is correct"
else
  echo "Formatting is incorrect, fixing..."
  cargo fmt --all
fi

# Lint
run_cmd "cargo $CARGO_Q clippy --all -- -D warnings"

# Test
run_cmd "cargo $CARGO_Q nextest run --locked"

# Build (release)
# Build CLI (needed for Tauri sidecar staging)
run_cmd "cargo $CARGO_Q build -p website-searcher --release"

# Linux packaging (Tauri) — always build AppImage; add deb/rpm per distro or flags
# Usage: ./compile.sh [-d|--deb] [-r|--rpm] [-v|--verbose] [-l|--log]

# (Flags already parsed above)

# Helper to add a bundle to a comma-separated list without duplicates
add_bundle() {
  local name="$1"
  case ",$BUNDLES," in
    *",${name},"*) ;;
    *)
      if [ -z "$BUNDLES" ]; then BUNDLES="$name"; else BUNDLES="$BUNDLES,$name"; fi
      ;;
  esac
}

# Determine base bundles (always AppImage)
BUNDLES=""
add_bundle "appimage"

# Autodetect distro family from /etc/os-release (best-effort)
if [ -r /etc/os-release ]; then
  # shellcheck disable=SC1091
  . /etc/os-release
  DETECT_SRC="${ID:-}${ID_LIKE:+ ${ID_LIKE}}"
  printf "Detected distro: %s\n" "$DETECT_SRC"
  printf "Auto-selecting bundles based on distro...\n"
  printf "%s" "$DETECT_SRC" | grep -Eqi '(debian|ubuntu)' && add_bundle "deb"
  printf "%s" "$DETECT_SRC" | grep -Eqi '(rhel|fedora|suse)' && add_bundle "rpm"
fi

# Manual overrides add to the set
[ "$WANT_DEB" -eq 1 ] && add_bundle "deb"
[ "$WANT_RPM" -eq 1 ] && add_bundle "rpm"

# If cargo-tauri is missing, skip bundling with a hint
if ! cargo tauri --version >/dev/null 2>&1; then
  echo "cargo-tauri not found; skipping bundling. Install with: cargo install tauri-cli --locked"
  exit 0
fi

# Stage Tauri sidecar (CLI) expected at src-tauri/bin/website-searcher-<host_triple>
TRIPLE=$(rustc -Vv | awk '/host:/{print $2}')
mkdir -p src-tauri/bin
cp -f target/release/website-searcher "src-tauri/bin/website-searcher-$TRIPLE"

# Normalize Linux maintainer scripts and helper before bundling
normalize_script() {
  local f="$1"
  [ -f "$f" ] || return 0
  # remove CR if present
  sed -i 's/\r$//' "$f" || true
  # ensure shebang exists; do not overwrite if present
  if ! head -n1 "$f" | grep -Eq '^#!/'; then
    # default to /bin/sh
    printf '%s\n' '#!/bin/sh' | cat - "$f" >"$f.tmp" && mv "$f.tmp" "$f"
  fi
  chmod +x "$f" || true
}

normalize_script "src-tauri/scripts/linux/postinst.sh"
normalize_script "src-tauri/scripts/linux/prerm.sh"
normalize_script "scripts/ws"

echo "Tauri bundles to build: $BUNDLES"
(
  cd src-tauri
  export APPIMAGE_EXTRACT_AND_RUN=1
  if [ "$VERBOSE" -eq 1 ] && [ "$LOG_ENABLED" -eq 1 ]; then
    cargo $CARGO_Q tauri build --bundles "$BUNDLES" 2>&1 | tee -a "../$LOG_FILE"
  elif [ "$VERBOSE" -eq 1 ]; then
    cargo $CARGO_Q tauri build --bundles "$BUNDLES"
  elif [ "$LOG_ENABLED" -eq 1 ]; then
    cargo $CARGO_Q tauri build --bundles "$BUNDLES" 1>>"../$LOG_FILE" 2> >(tee -a "../$LOG_FILE" >&2)
  else
    cargo $CARGO_Q tauri build --bundles "$BUNDLES" 1>/dev/null
  fi
)

exit 0