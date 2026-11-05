# 190 · Viewport splines: drawing, points, handles and dragging

Status: ready · Area: E · Depends on: 028, 143 · Observations: G3 · Touches: viewport, ui, command

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
- [ ] Splines show in the viewport and in screenshots; points/handles editable by drag.
- [ ] A G3 item asking about handle-link behaviour exists in docs/parity/GUI-SESSIONS.md (planner adds it; implementer checks).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Files changed, tests added, parity-pending rules, deviations, last gate lines.
