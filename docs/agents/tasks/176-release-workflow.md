# 176 · Release workflow: AppImage, dmg, zip/MSI, checksums, changelog

Status: done (pending first tagged run) · Area: N · Depends on: 173 · Observations: – · Touches: .github

## Goal
A tag builds installable packages for all three OS.

## Inputs
- Area design: [../../roadmap/areas/N-platform.md](../../roadmap/areas/N-platform.md)
- docs/roadmap/DECISIONS.md (D11)

## Design decisions (planner)
- cargo-dist or own scripts; AppImage bundles libxkbcommon-x11.

## Done when
- [ ] A test tag produces all artifacts and checksums. (Pending: needs a tagged or dispatched run on GitHub; Linux path verified locally.)
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `.github/workflows/release.yml` (prepare, linux, macos, windows, publish jobs; trigger `v*` tag or workflow_dispatch with `dry_run`, default true). No Rust tests added (no code change). Locally: built the four release binaries, ran `package-linux.sh` end to end (tar.gz + AppImage, appimagetool 1.9.1 downloaded), ran the AppImage (`AppImage forge` reaches the CLI), `checksums.sh`, `changelog.sh` against a temp clone with tags, `shellcheck -x -s sh` clean, YAML parses. Parity: n/a.
- Rules derived: tag must equal workspace.package version (checked in `prepare`). Changelog is `git log --no-merges` from previous `v*` tag, grouped Fixes / Features and changes.
- Deviations: own scripts instead of cargo-dist (scripts run locally). Windows MSI NOT built: zip only, MSI documented as follow-up in docs/RELEASING.md. Icons are rasterised by a Pillow script that mirrors the SVG geometry (no SVG renderer available); outputs committed. No brief 005 report exists; libxkbcommon, libxkbcommon-x11 and libxcb-xkb bundled by judgement. The Windows exe has no embedded icon (needs a build script).
- Files changed: .github/workflows/release.yml, scripts/release/* (lib.sh, version.sh, changelog.sh, checksums.sh, package-linux.sh, package-macos.sh, package-windows.ps1, sign-hooks.sh/.ps1, gen-icons.py), packaging/icons/*, docs/RELEASING.md, .gitignore (dist/), this brief, roadmap/BACKLOG.md.
- Open questions: only a tagged/dispatched run verifies the macOS (lipo, hdiutil), Windows (pwsh zip), softprops upload and ubuntu-22.04 build steps; whether `--locked` and the apt package list suffice on the runners.

