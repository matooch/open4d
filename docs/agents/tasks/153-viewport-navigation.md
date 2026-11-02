# 153 · Viewport navigation parity (keys, mouse, frame commands, views)

Status: done (parity-pending G2) · Area: L · Depends on: 152 · Observations: G2 · Touches: viewport, ui

## Goal
Alt+mouse orbit/pan/dolly, 1/2/3 key drags, scroll, frame all/selected, view switching, matching G2.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)
- GUI session G2 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G2 §2

## Design decisions (planner)
- Navigation as pure camera functions with unit tests; directions from G2 (parity-pending until answered).

## Done when
- [x] Unit tests for each gesture.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: status done (parity-pending G2). New tests: `crates/viewport/tests/nav.rs` (10: orbit keeps pivot and
  distance, pitch clamp, pan in the view plane and content follows the cursor, dolly scale and inverse,
  zoom-at keeps the point under the cursor fixed for perspective/orthographic/left-handed/off-centre cursors,
  frame fits all 8 corners for wide and tall aspects, point-frame recentres only, standard views, perspective
  round trip) and `crates/ui/tests/navigation.rs` (11: input mapping, `apply_nav`, frame all/selected, null and
  empty scene, `view.set_view`, `view.set_camera` + validation, `view.get_camera`, no undo steps, and
  egui_kittest Alt+left/middle/right drags, hold-key drag, wheel zoom to cursor, H/S/Mod+4/Mod+1 keys).
  No golden data involved (no parity numbers).
- Rules (all parity-pending G2, own design; VIEWPORT.md §7, UI.md §6.1, COMMANDS.md §6): Alt+left orbit
  (0.01 rad/px), Alt+middle or middle pan, Alt+right dolly (exp(-0.01 dx)), wheel zoom toward the cursor
  (0.999^points) about the pivot-plane point under the cursor, frame = bounding sphere with margin 1.1 and
  aspect-aware, seven named views (axis views are orthographic). Keys through the key map: H frame all, S frame
  selected, Mod+1..7 views. Commands: `view.frame_all` (now fits geometry), `view.frame_selected`,
  `view.set_view`, `view.set_camera`, `view.get_camera`. Undo decision: camera moves are not undo steps (view
  state, `undoable` false, tested).
- Deviations: (1) the brief's 1/2/3 + drag is replaced by hold keys X pan / C dolly / V orbit because 1..5 are
  the shading keys of the key map (pressing 1 would change shading); the hold keys are read from keyboard state,
  not key-map chords (not rebindable yet). (2) F1..F5 views of the area doc collide with F2 rename: views use
  Mod+1..7. (3) The 4-view toggle is not done (needs a multi-viewport panel; follow-up). (4) Zoom toward the
  cursor uses the pivot plane, not the scene depth under the cursor. (5) `Viewport unavailable` label made
  non-selectable (a selectable label stole the pointer from navigation in no-GPU tests) and the drag delta is
  read before locking the input (a nested read lock deadlocked under a drag).
- UI tests changed: `crates/ui/tests/models.rs::navigation_bindings` (NavInput gained fields; Orbit/Dolly now
  carry raw pixels and the wheel is `ZoomAt`; the radian maths moved to `forge_viewport::nav` tests);
  `crates/ui/tests/keymap.rs::default_bindings_table` (nine new default rows).
- Files changed: crates/viewport/src/{nav.rs,lib.rs}, crates/viewport/tests/nav.rs, crates/command/src/{state,context,lib}.rs,
  crates/command/src/builtin/object.rs, crates/ui/src/{navigation.rs,viewport_panel.rs,app.rs,commands.rs,state.rs,lib.rs},
  crates/ui/keymap/default.toml, crates/ui/tests/{navigation,models,keymap}.rs, docs/specs/{VIEWPORT,UI,COMMANDS}.md,
  docs/manual/reference/{commands,mcp}.md, BACKLOG.md, this brief.
- Open questions (G2 §2): drag directions and speeds, the real 1/2/3 behaviour and keys for each view, whether the
  wheel zooms toward the cursor, and the 4-view toggle key.
- Gate: `GATE: 923 tests passed, 0 failed` ... `cleanroom: ok` ... `GATE: all passed`.
