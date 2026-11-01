# 175 · Cross-platform settings, paths, fonts, file dialogs, modifiers

Status: done · Area: N · Depends on: 173 · Observations: – · Touches: platform (new), rpc, ui

## Goal
Correct per-OS config/cache/data paths, bundled fonts, native file dialogs, modifier handling.

## Inputs
- Area design: [../../roadmap/areas/N-platform.md](../../roadmap/areas/N-platform.md)

## Design decisions (planner)
- `directories` crate; `rfd` dialogs; bundled open-licence font.

## Done when
- [x] Tests for path resolution per OS in CI.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: spec `docs/specs/PLATFORM.md`; new crate `forge-platform` (`crates/platform`, uses `directories` 6) with pure `resolve(os, env, base)`, `settings_path_with`, `legacy_settings_path`, `migrate_legacy_settings` and process wrappers `dirs()` / `settings_path()`. 11 unit tests (every OS layout on every host, XDG override / empty / relative, FORGE_SETTINGS precedence, no-home, legacy path, migration copy / no-overwrite / no-op, plus cfg-gated tests for the host OS that run on all three CI OS). `crates/ui/tests/fonts.rs`: 2 tests (symbols missing before, covered after the fallback is installed; mac chord signs). Dialog stubs: existing `crates/ui/tests/files.rs` (`Dialogs` trait + `set_dialogs`). No parity cases.
- Rules: config = `$XDG_CONFIG_HOME|~/.config`/forge3d (Linux), `~/Library/Application Support/forge3d` (macOS), `%APPDATA%\forge3d` (Windows); data/cache analogous (PLATFORM.md §1). Precedence `FORGE_SETTINGS` > `XDG_*` (Linux, non-empty and absolute) > OS default. Migration: a pre-175 `~/.config/forge3d/settings.json` is copied (not moved) to the new path on macOS/Windows when the new file is absent; no-op on Linux (same path).
- Deviations: separate crate instead of a core module (keeps core pure; rpc must not depend on ui). `rfd` and the `Dialogs` trait already existed (briefs 009/IO), so only verified. Modifier display (Cmd vs Ctrl) from brief 152 verified (`Chord::display` uses egui `format_shortcut`). Font: egui 0.37 draws tofu in the app and panics only with `MissingGlyphPolicy::Panic`; bundled `crates/ui/assets/fonts/ForgeSymbols.ttf`, a 171 KB subset of DejaVu Sans (Bitstream Vera licence, notice in `LICENSE-DejaVu.txt`), registered as last fallback in `theme::install_fonts` (called from `Theme::apply`). A font file is not a crate, so `deny.toml` is unchanged. Button glyphs stay ASCII.
- Files changed: crates/platform/*, crates/rpc/{Cargo.toml,src/config.rs}, crates/ui/{Cargo.toml,src/lib.rs,src/theme.rs,tests/fonts.rs,assets/fonts/*}, docs/specs/{PLATFORM,RPC,UI}.md, docs/manual/automation.md, BACKLOG, this brief, Cargo.lock.
- Open questions: none.
- Gate: see commit message / final report.
