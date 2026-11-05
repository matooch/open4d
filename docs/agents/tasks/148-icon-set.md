# 148 · Icon set and icon atlas

Status: done · Area: L · Depends on: 147 · Observations: G1 · Touches: ui

## Goal
Our own SVG icon set for every command and node type, rendered into an atlas.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)
- GUI session G1 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G1 (layout only; never trace icons)

## Design decisions (planner)
- `crates/ui/icons/*.svg`, drawn by us (simple geometric glyphs); resvg → atlas at the UI scale; missing icon falls back to a letter glyph.
- Cleanroom check: no file may come from the reference app.

## Done when
- [x] Every registered command/node has an icon or a documented fallback; atlas test.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 7 tests in `crates/ui/tests/icons.rs` (every node type and command resolves to an existing glyph; map sorted/unique; folder matches compiled list, no unused file; shared SVG form (the cleanroom scan covers names); atlas size, inked glyphs, empty padding; scale, determinism, clamping; uv and ColorImage; letter fallback). No parity cases (no golden data); all 7 pass.
- Rules: UI.md §25. 160 glyphs drawn by us (24x24, white 1.5 strokes, tint mask); 62 node types and 141 commands = 203 ids mapped, 0 on the letter fallback today (the fallback exists for plugin types and later commands, tested with a synthetic id). Atlas: resvg (no text/font features), 16 columns, 1 px padding, premultiplied RGBA. All glyph-to-command assignments are parity-pending (G1).
- Deviations: new dependency `resvg 0.49` (default-features off; brings usvg, tiny-skia; `cargo deny check` passes). The icons are not yet drawn in the command bar/tool palette buttons (the layout snapshot baselines would change); `IconAtlas::texture(ctx)` and `icon_ref` are ready for that follow-up. Table in `src/icon_table.rs` is generated data.
- Files: crates/ui/icons/*.svg (160), crates/ui/src/icons.rs, crates/ui/src/icon_table.rs, crates/ui/src/lib.rs, crates/ui/tests/icons.rs, crates/ui/Cargo.toml, Cargo.lock, docs/specs/UI.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions: G1 answer for icon placement and size; whether buttons should switch from the text glyph to icons.
- Gate: `GATE: all passed`.
