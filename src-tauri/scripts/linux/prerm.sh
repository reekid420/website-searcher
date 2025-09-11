#!/bin/sh
set -e

# Remove ws and website-searcher symlinks
rm -f /usr/local/bin/ws || true
rm -f /usr/local/bin/website-searcher || true

exit 0


