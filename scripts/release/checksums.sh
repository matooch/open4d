#!/bin/sh
# Write dist/SHA256SUMS for every release artifact in dist/.
#   sh scripts/release/checksums.sh [DIST_DIR]
set -eu
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
[ $# -ge 1 ] && DIST=$1
cd "$DIST"
: > SHA256SUMS
for f in *; do
  case "$f" in
    SHA256SUMS|CHANGELOG.md|*.sha256) continue ;;
  esac
  [ -f "$f" ] || continue
  printf '%s  %s\n' "$(sha256_of "$f")" "$f" >> SHA256SUMS
done
cat SHA256SUMS
