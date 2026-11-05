# 190 · Viewport splines: drawing, points, handles and dragging

Status: done · Area: E · Depends on: 028, 143 · Observations: G3 · Touches: viewport, ui, command

## Goal
Draw spline objects (primitives and `forge.spline`) in the viewport, and in Points mode draw and
drag spline points and Bezier handles, through the brief 028 `spline.*` commands.

## Design decisions (planner)
- Splines draw as polylines from the same sampling the generators use (Intermediate Points), in the object colour; selected splines in the selection colour.
- Points mode on a `forge.spline`: points as squares, handles as small dots joined to their point; selected points highlighted.
- Picking reuses the viewport picking module (screen distance); a drag is one `spline.edit_points` gesture (one undo step). Snapping (brief 032) applies to point drags.
- Handle-link behaviour while dragging (mirrored vs broken tangents, Shift to break) is **parity-pending** until GUI session G3 answers it; default: mirrored.
- Headless tests: draw-list contents per display mode, picking, and a scripted drag producing exactly one undo step; one GPU baseline image with the lavapipe tolerance rule of the viewport tests.

## Done when
- [x] Splines show in the viewport and in screenshots; points/handles editable by drag.
- [x] A G3 item asking about handle-link behaviour exists in docs/parity/GUI-SESSIONS.md (planner adds it; implementer checks).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Files changed: `crates/viewport/src/curves.rs` (new: draw list, picking), `render.rs` (`LineSet`,
  `Frame.lines`), `lib.rs`; `crates/generators/src/splines.rs` (`sampled_loops`);
  `crates/ui/src/spline_edit.rs` (new: draw data, selection, `SplineDrag`), `viewport_panel.rs`,
  `screenshot.rs`, `state.rs`, `app.rs`, `lib.rs`; every `Frame { .. }` literal gained `lines: &[]`;
  docs: VIEWPORT.md section 11, SPLINES.md section 6, BACKLOG.md.
- Tests added: `crates/viewport/tests/curves.rs` (11, pure: draw list per display mode, picking),
  `display.rs` (2 GPU: all modes show the curve, baseline `spline_points.png` with the lavapipe
  tolerance rule, unchanged), `crates/ui/tests/spline_viewport.rs` (11: draw data, selection, one-undo-step
  drags, mirrored and broken handles, snapping and quantizing, screenshots).
- Parity-pending (G3 item 9, present in GUI-SESSIONS.md): handle link while dragging (default mirrored,
  Shift at the press breaks it), whether a point drag moves its handles (they travel, as offsets), sizes
  and colours of points and handles. Handles are not selectable; handles are not snapped.
- Deviations: splines draw with the depth-tested 1 px line pipeline (no thickness); Bounding Box mode draws
  the box of the curve. Primitives show their curve but are edited after Make Editable.
- Last gate lines: `cleanroom: ok`, `ok: 716 cases`, `GATE: all passed`.
