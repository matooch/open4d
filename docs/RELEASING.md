# Releasing

Source: DECISIONS D11 and brief 176. Workflow: `.github/workflows/release.yml`.

## Cutting a release
1. Set `workspace.package.version` in `Cargo.toml` (e.g. `0.2.0`), commit.
2. Tag `v0.2.0` and push the tag. The workflow refuses a tag that differs from the Cargo version.
3. The `publish` job creates the GitHub Release with the packages, `SHA256SUMS` and a changelog.

Dry run: Actions, "release", Run workflow, `dry_run` = true (default). Everything is built and
uploaded as workflow artifacts (`release-bundle` holds the final set); no Release is created.

## Artifacts
| OS | Files | Built by |
|---|---|---|
| Linux x86_64 (ubuntu-22.04) | `forge3d-V-linux-x86_64.tar.gz`, `.AppImage` | `scripts/release/package-linux.sh` |
| macOS universal2 | `forge3d-V-macos-universal2.dmg` (Forge3D.app, bundle id `org.forge3d.studio`), `-cli.tar.gz` | `package-macos.sh` (lipo of x86_64 + aarch64) |
| Windows x86_64 | `forge3d-V-windows-x86_64.zip` | `package-windows.ps1` |
| all | `SHA256SUMS`, `CHANGELOG.md` (release body) | `checksums.sh`, `changelog.sh` |

Each package holds `forge-studio` (editor), `forge`, `forge-mcp`, `forge-rpc-serve`.
The AppImage bundles `libxkbcommon`, `libxkbcommon-x11` and `libxcb-xkb` (loaded at run time by
winit); GPU drivers, libX11 and Wayland client libraries come from the host. Run
`Forge3D.AppImage forge ...` to use a bundled tool instead of the editor.

## Running locally
```
cargo build --release -p forge-studio -p forge-cli -p forge-mcp -p forge-rpc
sh scripts/release/package-linux.sh            # SKIP_APPIMAGE=1 for tar.gz only
sh scripts/release/checksums.sh                # dist/
sh scripts/release/changelog.sh v0.2.0         # notes to stdout
```
`appimagetool` 1.9.1 is downloaded and cached unless `APPIMAGETOOL` points at one. macOS and
Windows scripts must run on those systems. `dist/` is git-ignored.

## Icons
`packaging/icons/forge3d.svg` is original placeholder art. `scripts/release/gen-icons.py`
(Pillow) regenerates the committed PNG/ICO/ICNS files from the same geometry.

## Signing (brief 177)
`sign-hooks.sh` / `sign-hooks.ps1` are called at the right points and skip when secrets
(`MACOS_CERT_P12`, `WINDOWS_CERT_PFX`) are absent. Brief 177 implements them.

## Follow-ups
- Windows MSI (WiX v4 or cargo-wix) is not built yet; only the zip ships. Needs a `.wxs` with
  upgrade code, Start-menu shortcut and the `forge` CLI on PATH; test on a Windows runner.
- Embedding the .ico in `forge-studio.exe` needs a build script (winres-style); not done.
- Flatpak manifest is planned for v1.1 (D11).
