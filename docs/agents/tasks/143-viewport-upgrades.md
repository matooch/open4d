# 143 · Viewport upgrades: lights, shadows, AA, display modes, 4-view layout

Status: done (parity-pending G8) · Area: K · Depends on: – · Observations: G8 · Touches: viewport

## Goal
Scene lights in the viewport, shadow maps, MSAA, the display/shading modes and the 4-view layout.

## Inputs
- Area design: [../../roadmap/areas/K-rendering.md](../../roadmap/areas/K-rendering.md)
- GUI session G8 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G8 §1–2
- docs/specs/VIEWPORT.md

## Design decisions (planner)
- Light uniform buffer, one shadow map per light (cap 8), shading modes named per G8 (parity-pending).

## Done when
- [x] Baseline image tests per mode.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: status done (parity-pending G8). No golden data or parity numbers: G8 is unanswered, so every
  mode name, key, arrangement and default below is our own design (specs VIEWPORT.md §8-§9, UI.md §6.2,
  COMMANDS.md §6). New tests: `crates/viewport/tests/lights.rs` (7, pure: constructors, headlight = the
  legacy light, world-anchored rig with the key casting, cap of 8, directional and spot shadow cameras cover
  the bounds, point/non-casting give none), `tests/panes.rs` (6, pure: visible panes, 2x2 tiling with gaps
  for odd/even sizes, one pane = whole area, `pane_at`, default per-pane cameras), `tests/display.rs`
  (12, software adapter or `FORGE_REQUIRE_GPU=1`: constant ignores lights, hidden line hides far edges,
  bounding box, quick = headlight, spot cone, point range falloff, shadow under a box (and spot shadow, and
  none without `cast_shadow`), 10 lights = 8, MSAA edge pixels, 9 mode baselines + 7 lighting/MSAA baselines),
  `crates/ui/tests/panes.rs` (14: initial panes, `view.layout`, `view.maximize`, per-pane `view.shading` and
  `view.set_view`, activation keeps cameras, untouched panes follow the perspective pane, File > New reset,
  settings builder and scene-light hook, four-view screenshot composite, and egui_kittest: F4 layout toggle,
  Mod+M maximise and restore, hover activates a pane and Alt-drag navigates only it, View menu entries).
- Rules: lit colour `base * (0.25 + sum(color*intensity*atten*|n.l|*shadow))`, up to 8 lights; `Headlight`
  (default) is exactly the old `0.25 + 0.75|n.v|`, so existing images did not change; `Rig` = three
  world-anchored directionals (key 0.50 casts, fill 0.20, rim 0.10). Shadows: one 1024 `Depth32Float` array
  layer per casting light (cap 8), orthographic fit to the scene bounding sphere (directional) or perspective
  (spot), slope-scaled depth bias, 3x3 PCF, outside the map = lit. MSAA 4x via `Renderer::with_samples` +
  `RenderTargets` (resolve), `Settings.msaa` for offscreen. New modes: quick (headlight only), constant,
  hidden line, bounding box. Panes: `Perspective | Top | Front | Right`; four-view = 2x2 [persp, top / right,
  front] with a 2 px gap; ortho panes start as `Wire`; the pane under the pointer is active; `EditorState.camera`
  and `.shading` remain the active pane's working copy (so every existing tool and test is unchanged).
- Commands: `view.layout {layout?}` (absent toggles), `view.maximize {pane?}`, `view.shading` and
  `view.set_view` gain `pane?`, plus `view.lighting {mode}`, `view.shadows {enabled}`, `view.antialias {enabled}`.
  Keys (key map, all parity-pending G8): `6`..`9` = quick, constant, hidden_line, bounding_box; `F4` layout toggle;
  `Mod+M` maximise active pane. View menu: nine shading modes, layout, maximise, lighting, shadows, anti-aliasing.
- Light objects: no light node type exists yet. The viewport takes plain `Light` values; `forge_ui::panes::scene_lights`
  is the single hook (empty today) that will map light nodes to `Light` (VIEWPORT.md §8.1); with lights present they
  override the headlight/rig. The tests use `Lighting::Scene` with test lights.
- Baselines (our own images, not reference renders): `crates/viewport/tests/baselines/*.png`, 160x120, generated
  on lavapipe with `FORGE_UPDATE_BASELINES=1 cargo test -p forge-viewport --test display`; compared with mean
  error <= 1.0 and <= 0.3 % pixels off by more than 4 on a software adapter (looser, 4.0 / 3 %, on hardware).
- Deviations: (1) point lights cast no shadows (needs a cube map; documented, `cast_shadow` ignored for them).
  (2) isoparms and skeleton display modes are not implemented (no spline parameter lines or joints are drawn
  yet). (3) `Settings` stays `Copy`, so scene lights are a fixed array of 8 inside `Lighting::Scene` (one
  `allow(clippy::large_enum_variant)`). (4) `Renderer::render` gained a `resolve` parameter (only the panel
  and `render_to_rgba` call it). (5) `viewport.screenshot` renders what the viewport shows, including the four-pane
  composite, and now uses the editor's lighting and 4x MSAA.
- UI tests changed: `crates/ui/tests/keymap.rs::default_bindings_table` (six new default rows). Nothing else.
- Open questions (for a G8 form): names and order of the display modes; which view sits in which corner of the
  4-view layout and whether the orthographic panes start as wireframe; whether middle-click or a key maximises a pane.
- Files changed: crates/viewport/src/{lights,panes,render,lib}.rs, render.wgsl, tests/{lights,panes,display}.rs,
  tests/baselines/*.png; crates/command/src/{state,context,lib,builtin/object}.rs; crates/ui/src/{panes,app,state,
  commands,navigation,screenshot,viewport_panel,lib}.rs, keymap/default.toml, tests/{panes,keymap}.rs; docs/specs/
  {VIEWPORT,UI,COMMANDS}.md, docs/manual/reference/*, this brief and BACKLOG.md.
- Gate output: `GATE: 978 tests passed, 0 failed` / Python client OK (38 tests) / `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`.
