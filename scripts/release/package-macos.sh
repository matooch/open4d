#!/bin/sh
# Package macOS universal2: Forge3D.app inside a .dmg (runs on macOS only).
#   sh scripts/release/package-macos.sh X86_64_BIN_DIR AARCH64_BIN_DIR
# Each dir holds the four release binaries for that target, e.g.
# target/x86_64-apple-darwin/release and target/aarch64-apple-darwin/release.
# Binaries are merged with `lipo -create`. Output: dist/forge3d-VERSION-macos-universal2.dmg
# (plus a .tar.gz of the CLI tools). Signing/notarization: see sign-hooks.sh (brief 177).
set -eu
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

[ $# -eq 2 ] || die "usage: package-macos.sh X86_64_BIN_DIR AARCH64_BIN_DIR"
X86=$1; ARM=$2
STEM="forge3d-$VERSION-macos-universal2"
mkdir -p "$DIST"

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
UNI="$WORK/universal"
mkdir -p "$UNI"
for b in $BINS; do
  [ -f "$X86/$b" ] && [ -f "$ARM/$b" ] || die "missing $b in $X86 or $ARM"
  lipo -create -output "$UNI/$b" "$X86/$b" "$ARM/$b"
  lipo -info "$UNI/$b"
done

APP="$WORK/dmg/$APP_NAME.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
for b in $BINS; do cp "$UNI/$b" "$APP/Contents/MacOS/"; done
cp "$ROOT/packaging/icons/forge3d.icns" "$APP/Contents/Resources/forge3d.icns"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>$APP_ID</string>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleExecutable</key><string>forge-studio</string>
  <key>CFBundleIconFile</key><string>forge3d</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.graphics-design</string>
</dict>
</plist>
PLIST

# Signing hook (brief 177): signs "$APP" when the workflow prepared an identity.
sh "$(dirname "$0")/sign-hooks.sh" macos "$APP"

ln -s /Applications "$WORK/dmg/Applications"
hdiutil create -volname "$APP_NAME $VERSION" -srcfolder "$WORK/dmg" -ov -format UDZO "$DIST/$STEM.dmg"
echo "built $DIST/$STEM.dmg"

# CLI tools for people who do not want the app bundle.
mkdir -p "$WORK/cli/$STEM-cli/bin"
for b in forge forge-mcp forge-rpc-serve; do cp "$UNI/$b" "$WORK/cli/$STEM-cli/bin/"; done
tar -C "$WORK/cli" -czf "$DIST/$STEM-cli.tar.gz" "$STEM-cli"
echo "built $DIST/$STEM-cli.tar.gz"
