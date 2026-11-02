# Shared helpers for the release scripts. Source with: . "$(dirname "$0")/lib.sh"
# shellcheck disable=SC2034
# POSIX sh only.

ROOT=$(CDPATH="" cd -- "$(dirname -- "$0")/../.." && pwd)
APP_ID="org.forge3d.studio"
APP_NAME="Forge3D"
# Binaries shipped in every package: package -> binary name.
BINS="forge-studio forge forge-mcp forge-rpc-serve"

# Version from [workspace.package] in the root Cargo.toml.
workspace_version() {
  awk '
    /^\[workspace\.package\]/ { in_sec = 1; next }
    /^\[/ { in_sec = 0 }
    in_sec && /^version[ ]*=/ { gsub(/[" ]/, "", $0); sub(/^version=/, "", $0); print; exit }
  ' "$ROOT/Cargo.toml"
}

VERSION=${FORGE_VERSION:-$(workspace_version)}
[ -n "$VERSION" ] || { echo "cannot determine workspace version" >&2; exit 1; }

# Output directory for finished artifacts (default dist/, git-ignored).
DIST=${DIST:-$ROOT/dist}

die() { echo "error: $*" >&2; exit 1; }

# sha256 of a file on Linux (sha256sum) or macOS (shasum).
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'
  else shasum -a 256 "$1" | awk '{print $1}'; fi
}
