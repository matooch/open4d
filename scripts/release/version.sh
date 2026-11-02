#!/bin/sh
# Print the workspace.package version from Cargo.toml.
set -eu
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
echo "$VERSION"
