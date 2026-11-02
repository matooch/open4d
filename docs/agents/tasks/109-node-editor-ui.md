# 109 · Node expression editor UI

Status: done (parity-pending G6) · Area: H · Depends on: 108, 150 · Observations: G6 · Touches: ui

## Goal
A node canvas: add nodes, connect ports, pan/zoom; shared with node materials (124).

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)
- GUI session G6 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G6 §4

## Design decisions (planner)
- Own canvas or egui-snarl (MIT) – pick one and record it in DECISIONS. Edits are commands.

## Done when
- [x] egui_kittest: add two nodes and connect them; undo.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 22 egui_kittest tests in `crates/ui/tests/node_editor.rs` (panel opens from Window menu and by double-clicking an expression tag; add node via the search menu and undo; connect by dragging (one step) and undo/redo; wire from a free input; refused type connection and refused cycle change nothing and show the reason on the canvas and status bar; green/red verdict on the hovered port while dragging; node drag is one undo step, position saved in the graph, undo/redo; whole-selection drag; pick up a link and drop on empty canvas disconnects; move a link to another input in one step, dropping it back is no step; context menus (link Disconnect, node Delete, Disconnect <input>); box select + Delete key (one step, links go, objects untouched) and undo; shift-click toggle; pan and zoom about the pointer without document edits; value edit with `graph.set_value` and undo; linked inputs have no editor; live values follow edits and the timeline; a failing graph shows its error; an expression built entirely in the editor moves the cube and undo takes it back), 12 pure unit tests in `node_canvas.rs`, 4 in `node_editor.rs`, 7 in `crates/nodes/tests/editor_commands.rs`. Gate: 1289 tests passed, 0 failed. No parity numbers (own design).
- Canvas decision (DECISIONS D17): own canvas, not `egui-snarl`. 0.12 is MIT OR Apache-2.0 (fits deny.toml) but needs `egui ^0.36`, the workspace is on 0.37 (same trap as `egui_dock`, brief 147), and its model owns the graph and applies edits itself, which does not fit registry commands, merged drags and positions stored in the tag's parameter.
- Rules (all own design, **parity-pending G6**; spec UI.md §17): generic `node_canvas` (`CanvasModel` trait, `CanvasState::show` returns `CanvasAction`s, pure layout / hit testing / curves) and `node_editor` (the expression model, panel, `run_action` that maps every action to one `graph.*` command). Outputs listed first, then inputs; wheel zooms about the pointer (0.25..2.5), middle drag or Alt/Space+drag pans; left drag on a node moves the selection, on empty canvas box-selects (Shift adds), from a port draws a wire, from a linked input picks the link up; right click or double click on empty canvas opens the searchable add menu from `graph.node_types`; Delete/Backspace over the canvas deletes selected nodes (the key map's `object.delete` is redirected while the pointer is over the canvas); inline editors for unlinked float/int/bool/string/vec3/colour inputs below the zoom 0.75 they become text; matrix, node and any inputs are read-only text. Live values: `graph.evaluate` dry run cached by (scene revision, frame, tag). Panel `Panel::NodeEditor` (centre tab, closed by default, Window menu), tag chooser, "Add expression tag".
- Reuse for node materials (brief 124): implement `node_canvas::CanvasModel` (`node_keys`, `node`, `links`, `check_link`, `menu_entries`) and map the returned `CanvasAction`s to material commands; nothing in the canvas knows about expressions.
- Commands added or extended (NODES.md §6, COMMANDS.md): new `graph.move` (absolute positions, gesture-merged), `graph.add_node` `pos`, `graph.remove` `nodes` list, `graph.connect` `release_node`/`release_port` (moves a link between inputs in one step), `graph.set_value` `gesture`, `graph.get` returns `pos`. `GraphNode.pos` (saved only when not `[0, 0]`, so existing graph text is unchanged).
- Deviations: (1) brief asked for node positions "saved in the graph": added `pos` to `GraphNode` and `graph.move`; the file format needs no version bump (field is optional and defaults). (2) Node types for the add menu come from `graph.node_types`, but port lists of existing nodes are read from the built-in `NodeRegistry` (the commands only know that registry too). (3) Text constants commit on focus loss (one undo step per committed text) rather than per keystroke. (4) No snapping, minimap, copy/paste, comments, groups or frames: not asked, G6 unanswered.
- Existing tests changed: none.
- Files changed: crates/nodes/src/{graph,commands}.rs, crates/nodes/tests/editor_commands.rs, crates/ui/Cargo.toml, crates/ui/src/{node_canvas,node_editor,dock,state,app,object_manager,lib}.rs, crates/ui/tests/node_editor.rs, docs/specs/{UI,NODES,COMMANDS}.md, docs/roadmap/{DECISIONS,BACKLOG}.md, docs/manual/reference/{commands,mcp}.md, this brief, Cargo.lock.
- Open questions (G6): how nodes are added in the classic editor (menu, palette, drag from a list), what the node and port look is (in words), whether ports of different types are colour-coded, and how a refused link is shown. Also whether the editor should offer copy/paste and grouping.
- Gate: `GATE: 1289 tests passed, 0 failed` ... `advisories ok, bans ok, licenses ok, sources ok` ... `cleanroom: ok` ... `ok: 471 cases` ... `GATE: all passed`.
