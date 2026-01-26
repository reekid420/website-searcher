#!/bin/sh
set -e

# Ensure ws and website-searcher are available on PATH via /usr/local/bin symlinks
# Prefer system-installed CLI under /usr/bin, fallback to /opt paths
CLI="/usr/bin/website-searcher"
if [ ! -x "$CLI" ]; then
  CLI="/opt/website-searcher/bin/website-searcher"
  [ -x "$CLI" ] || CLI="/opt/website-searcher/website-searcher"
fi

if [ -n "$CLI" ] && [ -x "$CLI" ]; then
  ln -sf "$CLI" /usr/local/bin/website-searcher || true
  ln -sf "$CLI" /usr/local/bin/ws || true
fi

exit 0


