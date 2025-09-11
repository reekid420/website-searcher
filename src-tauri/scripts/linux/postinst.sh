#!/bin/sh
set -e

# Create ws symlink pointing to the installed website-searcher binary
CLI="/opt/website-searcher/bin/website-searcher"
[ -x "$CLI" ] || CLI="/opt/website-searcher/website-searcher"

if [ -x "$CLI" ]; then
  ln -sf "$CLI" /usr/local/bin/website-searcher || true
  ln -sf "$CLI" /usr/local/bin/ws || true
fi

exit 0


