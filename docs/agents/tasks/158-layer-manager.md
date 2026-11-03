# 158 · Layer manager

Status: done (parity-pending G1/G8) · Area: L · Depends on: 149 · Observations: G1 · Touches: ui, core

## Goal
Layers with solo, view, render, manager, lock, animation, generators, deformers, expressions toggles.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)
- GUI session G1 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**

## Design decisions (planner)
- Layer is a document-level node; objects link to it.

## Done when
- [x] Toggling view hides objects in the viewport; save/load.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: spec first (CORE.md §6.8 and §7, UI.md §22, COMMANDS.md §6, TRANSFORM.md §2). Tests added: `crates/core/tests/layers.rs` (14: defaults, undo, link validation, hidden param, tags, view/solo/render resolution, per-node flags, delete, merge, save/load, bad layer data, dependency diff), `crates/core/tests/native_format.rs` (v8 fixtures migrate to 9, v9 fixtures `scene-v9*.forge` byte stable), `crates/app/tests/layers.rs` (13: commands and their one undo step each, bad arguments, selection default, view/solo/render, animation, generators pass-through, deformers, expressions, lock/manager do not evaluate, dirtiness by evaluated-node counts against a fresh scheduler, 60 seeded random layer edits against a fresh evaluation), `crates/ui/tests/layer_manager.rs` (16: pure toggle/merge/assign arguments, the Layer sub menu, row colours and manager filtering, view hides draw items, locked cannot be picked or made a component target; egui_kittest: Window menu, New layer, the nine toggle cells with undo, selection-wide toggles, rename, delete/merge/assign/remove buttons, object context menu, save/open). No parity cases (own design).
- Rules (own design, **parity-pending G1**): a layer is document level data (`Scene::layers()`), not a node; ids come from the node id counter. Objects link by the hidden `object.layer` Link param (a tag follows its host); membership is the node's own link, children do not inherit. view/solo resolve with the object's visibility in `visibility::editor_visible` (solo: only soloed layers show while any is soloed; unassigned objects hide; view off always hides), render in `render_visible`; locked: no viewport pick, no gizmo/axis/component drag, no component target; animation off: `forge_anim::overrides` skips members; expressions off: tags of members do not run; deformers off: `apply_deformer` returns early; generators off: the generator makes no geometry and its inputs are no longer consumed (drawn as themselves); manager off: row omitted, children listed one level up. Dirtiness: `depgraph::diff` compares `layers::effect` per node when the tables or a link differ and seeds only nodes whose effect changed (a lock, colour or rename seeds nothing). Commands: `layer.create`, `layer.delete`, `layer.rename`, `layer.set`, `layer.assign`, `layer.merge`. File format 8 to 9: `layers` block (omitted while empty) and `object.layer`; step 8 to 9 rewrites nothing; load drops repeated/clashing layer ids and clears dangling links with warnings.
- Deviations: (1) `object.layer` is a Link whose target is a layer id, not a node: `Tx::set_param` validates it against the layer table, the dependency graph skips it (no node has that id). (2) Adding the 4th `object.*` param changed counts in existing tests, as brief 149 did: `crates/core/tests/{registry,scene_edit,transform}.rs`; the saved-version asserts moved from 8 to 9 in `crates/core/tests/{file,tracks,time,native_format}.rs` and `crates/nodes/tests/expression.rs` (the "future version" cases 9 to 10); v8 fixtures stay as the old build wrote them. (3) Render visibility is resolved but the render bridge still does not read it (brief 149 follow-up). (4) Locked limits the viewport only; selecting in the object manager and editing attributes still work. (5) `TreeRow` lost `Eq` (it carries a colour). (6) Layers panel default area right bottom; a colour drag commits one `layer.set` when the pointer is released.
- Files changed: crates/core/src/{layers.rs,lib.rs,scene.rs,document.rs,visibility.rs,depgraph.rs,file.rs}, crates/anim/src/scene_at.rs, crates/nodes/src/expression.rs, crates/modifiers/src/lib.rs, crates/generators/src/{lib.rs,cloner.rs}, crates/app/src/select.rs, crates/command/src/builtin/{layer.rs,mod.rs}, crates/ui/src/{layer_manager.rs,tree.rs,object_manager.rs,dock.rs,state.rs,app.rs,manipulate.rs,lib.rs}, tests as listed above plus fixtures `crates/core/tests/fixtures/scene-v9*.forge`, docs/specs/{CORE,UI,COMMANDS,TRANSFORM}.md, docs/manual/reference/{commands,mcp}.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions (G1): which toggles the reference has and what each means for children of a member, whether solo also affects renders, what a bypassed generator leaves, whether a locked object may still be selected in the object manager, layer colour display and the column order.
- Gate: `GATE: 1597 tests passed, 0 failed` ... `cleanroom: ok` ... `ok: 471 cases` ... `GATE: all passed`.
