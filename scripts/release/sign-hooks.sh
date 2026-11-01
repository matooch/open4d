#!/bin/sh
# Signing hook points (brief 177 fills these in). Each hook is a no-op unless its secrets are
# present, so unsigned builds work out of the box (DECISIONS D11).
#   sh scripts/release/sign-hooks.sh macos|windows|linux PATH
set -eu
case "${1:-}" in
  macos)
    # HOOK(177): codesign + notarytool. Needs MACOS_CERT_P12 / MACOS_NOTARY_* secrets.
    if [ -z "${MACOS_CERT_P12:-}" ]; then echo "sign-hooks: macOS signing skipped (no secrets)"; exit 0; fi
    echo "sign-hooks: macOS signing not implemented yet (brief 177)"
    ;;
  windows)
    # HOOK(177): signtool. Needs WINDOWS_CERT_PFX secret. Windows runs scripts/release/sign-hooks.ps1.
    echo "sign-hooks: use sign-hooks.ps1 on Windows"
    ;;
  linux)
    # HOOK(177): optional detached GPG signature of the AppImage/tar.gz.
    echo "sign-hooks: Linux signing skipped (no secrets)"
    ;;
  *) echo "usage: sign-hooks.sh macos|windows|linux PATH" >&2; exit 2 ;;
esac
