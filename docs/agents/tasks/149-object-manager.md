# 149 · Object manager parity: columns, dots, tags, drag/drop, search, context menus

Status: done (parity-pending G1) · Area: L · Depends on: 147 · Observations: G1 · Touches: ui

## Goal
Object manager with visibility dots, enable tick, tag column, drag reparent, search and context menu.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)
- GUI session G1 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G1 §2

## Design decisions (planner)
- Tree logic as pure functions (drop target computation, multi-select ranges) with unit tests.

## Done when
- [x] egui_kittest: drag reparent, toggle dots, undo.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `crates/ui/tests/object_manager.rs` (26 tests). Pure: drop zones and drop targets for every zone, invalid drops (self, own subtree), moved set, click/Shift/Ctrl ranges with anchor, right-click rule, filter (name, type, tag, ancestors dimmed), context items and their toggling labels, visibility resolution (core), viewport mesh skipping. Commands: `object.move` (reorder, reparent, several nodes, cycle fails, one undo step), `object.group`, `object.visibility`, `object.enable`. egui_kittest, each with undo: reparent by drag, reorder by drag to a row edge, dragging a selected row moves the whole selection, Shift/Ctrl clicks, editor and render dots and the tick (cycle, selection-wide), double-click rename (Enter commits, Esc cancels), search, context menu (Group, Delete, Hide, Move to Root), a drag from the tree onto an attribute link field (brief 150 still works). No parity numbers (no golden data).
- Rules (all own design, **parity-pending G1**): columns expand arrow | name (+ type label) | tags | editor dot, render dot, enable tick (right aligned); dot click cycle Default -> Off -> On -> Default; drop zones top quarter / middle / bottom quarter of a row; context menu items Rename, Group Objects, Make Editable, Hide/Show, Disable/Enable, Move to Root, Delete. Spec: docs/specs/UI.md §13 (§3 now points to it), COMMANDS.md §6, TRANSFORM.md §2, CORE.md §2.
- Added: core `visibility` module and three hidden params on every hierarchy type (`object.editor_vis`, `object.render_vis`, `object.enabled`); commands `object.move`, `object.group`, `object.visibility`, `object.enable`. Existing `object.rename`, `object.reparent`, `object.delete`, `mesh.make_editable` are reused.
- Deviations: (1) No visibility state existed, so the params were added to the registry; they are hidden in the attribute manager (the dots are their UI). The viewport honours editor visibility and enable (hidden objects are not drawn); the render bridge does not read render visibility yet (follow-up). (2) The old drop-on-row-reparent and the empty-area drop now run `object.move`; `object.reparent` stays for agents. (3) A disabled ancestor also hides the node from `render_visible`. (4) Search disables dragging and the expand arrow while active. (5) Tag drag/drop and layer colours are not done. (6) kittest needs `max_double_click_delay` raised for double-clicks, since steps are 0.25 s apart.
- Existing tests changed: `crates/core/tests/registry.rs` (null has 6 params), `crates/core/tests/scene_edit.rs` (14 params), `crates/core/tests/transform.rs` (param order, `size` index 6). No UI test changed.
- Files: crates/core/src/{visibility.rs,lib.rs,registry.rs}, crates/command/src/builtin/object.rs, crates/ui/src/{object_manager.rs,tree.rs,app.rs,lib.rs,viewport_panel.rs}, crates/ui/tests/object_manager.rs, crates/core/tests/{registry,scene_edit,transform}.rs, docs/specs/{UI,COMMANDS,TRANSFORM,CORE}.md, docs/manual/reference/{commands,mcp}.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions (G1): column order and what the dots/tick show, dot colours and click cycle, context menu items, drop zone sizes, whether a drop keeps world transforms.
- Gate: `GATE: 879 tests passed, 0 failed` ... `cleanroom: ok` ... `GATE: all passed`.
