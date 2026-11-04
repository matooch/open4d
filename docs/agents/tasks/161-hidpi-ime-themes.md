# 161 · HiDPI, IME, themes, keyboard navigation

Status: done · Area: L · Depends on: 147 · Observations: – · Touches: ui

## Goal
Correct scaling, text input methods, a dark theme close to the reference feel and a light theme, keyboard focus navigation.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)

## Design decisions (planner)
- Theme tokens in one module.

## Done when
- [x] Tests at scale 1, 1.5, 2.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 13 tests added (`crates/ui/tests/accessibility.rs`); no parity data (own design), nothing in `UNSUPPORTED` / `KNOWN_MISMATCH`.
- Rules (spec UI.md §24): theme tokens for dark / light / high contrast in `theme` with a `focus` token and WCAG contrast tests; `UiPrefs` (theme, scale 0.5-3) in the `ui` key of the settings file; commands `ui.theme` and `ui.scale` (window requests, refused headless) and View > Theme / UI scale menus; scale is egui's zoom factor on top of the OS scale; focus ring = 2 pt `focus` stroke on the active widget style; IME underline uses `focus`; a system CJK font (if found) is added as last fallback; Tab / Shift+Tab are unbound in the key map. All parity-pending (no reference observation).
- Deviations: the component-selection colour passed to the GPU viewport still uses the dark token (that function has no egui context). Dark error colour contrast is only asserted >= 3 (existing palette not changed). egui shows the IME preedit inside the text field while composing (test asserts that, not "text only after commit").
- Files changed: crates/ui/src/{theme,prefs,app,state,lib,console,history_panel,keymap_editor,node_editor,palette,script_panel,timeline,viewport_panel}.rs, crates/command/src/{state.rs,builtin/file.rs}, crates/ui/tests/accessibility.rs, docs/specs/UI.md, docs/manual/reference/{commands,mcp}.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md.
- Open questions: real IME and HiDPI behaviour on a desktop is not exercised headless; a bundled CJK font was not added (licence / size).
- Gate: `GATE: all passed`.
