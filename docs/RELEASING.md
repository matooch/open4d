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

## Signing and notarization (brief 177)
Sources: public platform documentation for `codesign`, `notarytool`, `stapler`, `signtool` and
GnuPG. Every signing step runs only when its secrets exist. Without them the job logs
`signing skipped: <reason>` and uploads the unsigned artifacts; a missing secret never fails a
release. Secrets are read through step `env:` only (never interpolated into scripts, never
echoed). Temporary keychains, key files and GnuPG homes live in `$RUNNER_TEMP` and are deleted
by an `if: always()` step.

Each gate is a step output computed from `secrets.X != ''` (the `secrets` context cannot be used
in every `if:`): `steps.mac_gate.outputs.sign` / `.notarize`, `steps.win_gate.outputs.sign`,
`steps.gpg_gate.outputs.sign`.

| Secret | Used by | How to create it |
|---|---|---|
| `MACOS_CERT_P12` | macOS codesign | Export a "Developer ID Application" certificate with its private key from Keychain Access as `.p12`; `base64 -i cert.p12 | pbcopy`. |
| `MACOS_CERT_PASSWORD` | macOS codesign | The password chosen at export. |
| `MACOS_SIGN_IDENTITY` | optional | Identity name (`Developer ID Application: Name (TEAMID)`). If unset, the first Developer ID identity in the imported keychain is used. |
| `MACOS_NOTARY_KEY_P8` | notarization | App Store Connect, Users and Access, Integrations, create an API key; download `AuthKey_XXXX.p8`; `base64 -i AuthKey_XXXX.p8`. |
| `MACOS_NOTARY_KEY_ID` | notarization | The key id shown next to that API key. |
| `MACOS_NOTARY_ISSUER_ID` | notarization | The issuer id shown at the top of the keys page. |
| `WINDOWS_CERT_PFX` | Windows signtool | A code-signing certificate with private key as `.pfx`; `base64 -w0 cert.pfx`. |
| `WINDOWS_CERT_PASSWORD` | Windows signtool | The `.pfx` password. |
| `LINUX_GPG_PRIVATE_KEY` | AppImage / tar.gz signature | `gpg --export-secret-keys --armor KEYID | base64 -w0`. |
| `LINUX_GPG_PASSPHRASE` | optional | Key passphrase, if the key has one. |

What is covered:
- macOS: `scripts/release/sign-hooks.sh macos` signs every binary in `Forge3D.app` and the app
  itself (`codesign --force --timestamp --options runtime`, hardened runtime) before the dmg is
  built. The workflow then signs the `.dmg`, runs `xcrun notarytool submit --wait` with the API
  key and `xcrun stapler staple` on it. Without the notary secrets the dmg is signed only.
- Windows: `scripts/release/sign-hooks.ps1` runs `signtool sign /fd sha256 /tr <url> /td sha256`
  on each exe before the zip is made (timestamp server `WINDOWS_TIMESTAMP_URL`, default
  `http://timestamp.digicert.com`). The zip itself cannot be signed.
- Linux: a detached armored signature `NAME.asc` for the AppImage and the tar.gz, published next
  to them and listed in `SHA256SUMS`. Verify with `gpg --verify NAME.asc NAME`.
- Not covered: the CLI tarballs on macOS are not notarized (plain tar.gz); the MSI is not built.

The structure is checked by `apps/forge/tests/release_signing.rs` (gating, no echoed secrets,
`if: always()` cleanup, unsigned upload path). The real tools only run on tagged or dispatched
runs with secrets, so verify the first signed release by hand: `codesign -dv --verbose=4`,
`spctl -a -t open --context context:primary-signature -v X.dmg`, `xcrun stapler validate X.dmg`.

## Follow-ups
- Windows MSI (WiX v4 or cargo-wix) is not built yet; only the zip ships. Needs a `.wxs` with
  upgrade code, Start-menu shortcut and the `forge` CLI on PATH; test on a Windows runner.
- Embedding the .ico in `forge-studio.exe` needs a build script (winres-style); not done.
- Flatpak manifest is planned for v1.1 (D11).
