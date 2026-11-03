# 157 · HUD and viewport overlays

Status: done (parity-pending G1/G8) · Area: L · Depends on: 143 · Observations: G8 · Touches: viewport

## Goal
HUD (fps, polygon counts, selection info, safe frames).

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)
- GUI session G8 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G8 §4

## Design decisions (planner)
- Overlay draw list; toggles per viewport.

## Done when
- [x] Baseline image test.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: spec first (UI.md §23, COMMANDS.md §6 `view.hud`). `crates/ui/tests/hud.rs` (25 tests): pure (count formatting, safe frame geometry for every aspect case, the draw list's text and placement per item, the time display formats, the frame timer, settings defaults/round trip/leniency, counts of drawn objects and of the selection incl. hidden objects and an instanced cloner, the context built from the editor state, layouts with and without a HUD block), the `view.hud` command (toggle, pane, all panes, explicit values, errors, no undo steps), two **baseline images** of the draw list (`crates/ui/tests/baselines/hud_all.png`, `hud_safe_render.png`: a pure rasteriser, text as 6 x 10 boxes per glyph, so no fonts or GPU are involved; `FORGE_UPDATE_BASELINES=1` rewrites them), and egui_kittest (View > HUD > pane > item check boxes, only visible panes with items paint, four-view layout, counts follow selection and scene, frame item follows the timeline and display, layout save/restore, a workspace switch keeps the HUD, a frame rate reading after frames). No parity cases (own design).
- Rules (own design, **parity-pending G8**): six items per pane, all off by default: frame rate (smoothed `unstable_dt`), counts (objects, points, polygons: scene and selection, from the scheduled draw frame, instances counted by prototype geometry times visible clones), tool and snap (active modeling tool or gizmo mode, snap off/on 3D|2D, axis mode), frame (`format_time` in the scene's time display, over the project end), camera (pane name and projection), safe frames (render aspect fitted into the pane, action 90 %, title 80 %; the render size is `None` today because the scene has no render settings, so the frame is the pane). Placement: left column under the pane label (camera, tool, snap, counts), frame rate top right, frame bottom right. The draw list is pure (`hud::draw_list`) and the egui painter only maps primitives (`Editor::paint_hud`, also over the "Viewport unavailable" placeholder so headless tests see it). Settings: `EditorState.hud`, `view.hud` command and the View > HUD menu; saved with layouts through `Layout.hud: Option<HudSettings>` (serde default, `LAYOUT_VERSION` stays 1); workspaces carry `None` and keep the user's HUD.
- Deviations: (1) No camera objects exist, so "camera name" is the pane name and the projection of the pane's camera. (2) No scene render size yet: the safe frame code takes `Option<[u32; 2]>` and tests exercise it, the editor passes `None`. (3) The baseline is a rasterised draw list, not a screenshot of the egui output, because the egui text depends on fonts and the GPU path is not available in the test harness (the viewport's own baselines are in forge-viewport). (4) A new `CommandContext::set_hud` (headless contexts ignore it).
- Files changed: crates/ui/src/{hud.rs,app.rs,commands.rs,layout.rs,lib.rs,state.rs,workspaces.rs}, crates/command/src/{builtin/object.rs,context.rs,lib.rs,state.rs}, crates/ui/tests/{hud.rs,baselines/hud_all.png,baselines/hud_safe_render.png}, docs/specs/{UI,COMMANDS}.md, docs/manual/reference/{commands,mcp}.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions (G8): which items the reference HUD has and their wording and places, whether it can show the real GPU frame time, the safe frame percentages, whether the HUD is per view or global, and the defaults.
- Gate: `GATE: 1622 tests passed, 0 failed` ... `cleanroom: ok` ... `ok: 471 cases` ... `GATE: all passed`.
