#!/bin/sh
# Package Linux x86_64: tar.gz and AppImage.
#   sh scripts/release/package-linux.sh [BIN_DIR]
# BIN_DIR defaults to target/release (run `cargo build --release -p forge-studio -p forge-cli
# -p forge-mcp -p forge-rpc` first). Output goes to dist/ (override with DIST=...).
# Env: APPIMAGETOOL=path to appimagetool (downloaded and cached if unset),
#      SKIP_APPIMAGE=1 to build only the tar.gz.
#
# The AppImage bundles libxkbcommon and libxkbcommon-x11 (egui/winit loads them at run time)
# via ldd-resolved copies; the GPU driver (Vulkan/GL), libX11/Wayland client libs and libc stay
# system-provided, as is standard for AppImages.
set -eu
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

BIN_DIR=${1:-$ROOT/target/release}
ARCH=x86_64
STEM="forge3d-$VERSION-linux-$ARCH"
mkdir -p "$DIST"
for b in $BINS; do [ -x "$BIN_DIR/$b" ] || die "missing binary $BIN_DIR/$b"; done

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

# ---- tar.gz -------------------------------------------------------------------------------
mkdir -p "$WORK/tar/$STEM/bin"
for b in $BINS; do cp "$BIN_DIR/$b" "$WORK/tar/$STEM/bin/"; done
cp "$ROOT"/LICENSE* "$WORK/tar/$STEM/" 2>/dev/null || true
cp "$ROOT/packaging/icons/forge3d-256.png" "$WORK/tar/$STEM/forge3d.png"
cat > "$WORK/tar/$STEM/README.txt" <<TXT
Forge3D $VERSION (Linux $ARCH)
Run bin/forge-studio for the editor; bin/forge is the command-line tool.
The editor needs libxkbcommon-x11 and a Vulkan or OpenGL driver installed.
TXT
tar -C "$WORK/tar" -czf "$DIST/$STEM.tar.gz" "$STEM"
echo "built $DIST/$STEM.tar.gz"

[ "${SKIP_APPIMAGE:-}" = 1 ] && exit 0

# ---- AppImage -----------------------------------------------------------------------------
APPDIR="$WORK/Forge3D.AppDir"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/lib" "$APPDIR/usr/share/metainfo" \
         "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/256x256/apps"
for b in $BINS; do cp "$BIN_DIR/$b" "$APPDIR/usr/bin/"; done

# Bundle libxkbcommon + libxkbcommon-x11 (and xcb-xkb glue) next to the binary.
for lib in libxkbcommon.so.0 libxkbcommon-x11.so.0 libxcb-xkb.so.1; do
  p=$(ldconfig -p 2>/dev/null | awk -v l="$lib" '$1 == l && /x86-64/ { print $NF; exit }')
  if [ -n "$p" ] && [ -e "$p" ]; then cp -L "$p" "$APPDIR/usr/lib/$lib"
  else echo "warning: $lib not found on this machine, AppImage will rely on the host" >&2; fi
done

cat > "$APPDIR/AppRun" <<'RUN'
#!/bin/sh
HERE=$(dirname "$(readlink -f "$0")")
export LD_LIBRARY_PATH="$HERE/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PATH="$HERE/usr/bin:$PATH"
# First argument selects a bundled tool: `Forge3D.AppImage forge ...`; default is the editor.
case "${1:-}" in
  forge|forge-mcp|forge-rpc-serve) tool=$1; shift; exec "$HERE/usr/bin/$tool" "$@" ;;
  *) exec "$HERE/usr/bin/forge-studio" "$@" ;;
esac
RUN
chmod +x "$APPDIR/AppRun"

DESKTOP="[Desktop Entry]
Type=Application
Name=$APP_NAME
Comment=3D modelling and motion graphics
Exec=forge-studio %F
Icon=forge3d
Categories=Graphics;3DGraphics;
Terminal=false"
printf '%s\n' "$DESKTOP" > "$APPDIR/forge3d.desktop"
printf '%s\n' "$DESKTOP" > "$APPDIR/usr/share/applications/$APP_ID.desktop"
cp "$ROOT/packaging/icons/forge3d-256.png" "$APPDIR/forge3d.png"
cp "$ROOT/packaging/icons/forge3d-256.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/forge3d.png"
cat > "$APPDIR/usr/share/metainfo/$APP_ID.appdata.xml" <<XML
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>$APP_ID</id>
  <name>$APP_NAME</name>
  <summary>3D modelling and motion graphics</summary>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>MIT OR Apache-2.0</project_license>
  <releases><release version="$VERSION"/></releases>
</component>
XML

TOOL=${APPIMAGETOOL:-}
if [ -z "$TOOL" ]; then
  TOOL="${XDG_CACHE_HOME:-$HOME/.cache}/forge3d-release/appimagetool-1.9.1-x86_64.AppImage"
  if [ ! -x "$TOOL" ]; then
    mkdir -p "$(dirname "$TOOL")"
    curl -fsSL -o "$TOOL" \
      https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage
    chmod +x "$TOOL"
  fi
fi
# Run extracted so FUSE is not required (CI containers, sandboxes).
ARCH=$ARCH APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" --no-appstream "$APPDIR" "$DIST/$STEM.AppImage"
echo "built $DIST/$STEM.AppImage"
