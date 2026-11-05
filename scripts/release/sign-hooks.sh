#!/bin/sh
# Signing hook points (brief 177, docs/RELEASING.md). Each hook is a no-op unless the workflow
# prepared a signing identity, so unsigned builds work out of the box (DECISIONS D11).
#   sh scripts/release/sign-hooks.sh macos|linux PATH
# macOS: needs MACOS_SIGN_IDENTITY (and optionally MACOS_KEYCHAIN), set by the release workflow
#        after it imported the certificate into a temporary keychain. Signs the .app inside out
#        with the hardened runtime. The dmg, notarization and stapling are workflow steps.
# Windows: scripts/release/sign-hooks.ps1.  Linux: detached GPG signing is a workflow step.
set -eu
case "${1:-}" in
  macos)
    APP=${2:?usage: sign-hooks.sh macos APP}
    if [ -z "${MACOS_SIGN_IDENTITY:-}" ]; then echo "signing skipped: no macOS signing identity"; exit 0; fi
    KC=""
    [ -n "${MACOS_KEYCHAIN:-}" ] && KC="--keychain $MACOS_KEYCHAIN"
    # Sign nested code first, then the bundle (no --deep: it is deprecated for signing).
    for f in "$APP"/Contents/MacOS/*; do
      # shellcheck disable=SC2086
      codesign --force --timestamp --options runtime $KC --sign "$MACOS_SIGN_IDENTITY" "$f"
    done
    # shellcheck disable=SC2086
    codesign --force --timestamp --options runtime $KC --sign "$MACOS_SIGN_IDENTITY" "$APP"
    codesign --verify --strict --verbose=2 "$APP"
    echo "sign-hooks: signed $APP"
    ;;
  windows)
    echo "sign-hooks: use sign-hooks.ps1 on Windows"
    ;;
  linux)
    echo "signing skipped: Linux signatures are made by the workflow (LINUX_GPG_PRIVATE_KEY)"
    ;;
  *) echo "usage: sign-hooks.sh macos|windows|linux PATH" >&2; exit 2 ;;
esac
