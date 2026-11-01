# 025 · Bridge, Stitch and Sew, Close Polygon Hole, Polygon Pen

Status: ready · Area: B · Depends on: 020 · Observations: G3 · Touches: modeling, ui

## Goal
Topology-building tools: bridge two border loops or polygons, stitch, fill holes, and draw polygons point by point.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- GUI session G3 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- docs/parity/modeling-tools-2.md Notes (these returned false through the script; GUI only)

## Design decisions (planner)
- Pure functions: `bridge(mesh, loop_a, loop_b, segments)` (matches loop orientation by the shortest total edge length); `close_hole(mesh, border_edge)` (one N-gon); `stitch(mesh, pairs)`.
- Polygon Pen: click to add points or polygons and snap to existing points (uses snapping from 032 if present).
- Own design, parity-pending.

## Done when
- [ ] MODELING.md §6.
- [ ] Unit tests: bridging two open cube ends gives a closed tube; close_hole restores the face count.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added; parity N/M (`UNSUPPORTED` / `KNOWN_MISMATCH` with reasons).
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations from the design decisions, and why.
- Files changed.
- Open questions or data that would settle them (the reviewer turns these into follow-up forms).
- The last lines of the gate output.
