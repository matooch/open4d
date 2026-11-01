# 022 · Mesh commands: Subdivide, Triangulate, Untriangulate, Reverse, Optimize, Weld, Melt, Dissolve, Split, Disconnect, Connect, Mirror, Center Axis

Status: ready · Area: B · Depends on: 020 · Observations: O6, O11, O25 · Touches: modeling, ui

## Goal
The common mesh commands exist as registry commands and match the observed cases.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O6: [../../parity/modeling-tools.md](../../parity/modeling-tools.md) and its golden cases
- O11: [../../parity/followups-1.md](../../parity/followups-1.md) and its golden cases
- O25: [../../parity/modeling-tools-2.md](../../parity/modeling-tools-2.md) and its golden cases
- tests/golden/modeling-tools/ (O6, O11 §4, O25 cases) and the settings tables in docs/parity/modeling-tools.md and modeling-tools-2.md

## Design decisions (planner)
- One pure function per command in `forge-modeling`, one command each (`mesh.subdivide`, `mesh.triangulate`, …). Commands without readable settings in O25 use the documented values from the cases plus our own defaults, marked parity-pending.
- Subdivide with Hyper on: O25 shows the export is not rounded (bbox ±100), so check whether the smoothing applies only on display. Derive this from subdivide-smooth.
- Split creates a second object (`Cube.1`) holding a copy of the selected polygons (O25 split-top).
- Center Axis moves the object position to the bounding-box centre and compensates the points (O25 center-axis).
- Mirror with an explicit plane point and normal (O25 mirror-x). Duplicate off flips the winding.
- Dissolve and Close Polygon Hole had no scripted result in O25, so implement them as our own design, marked parity-pending (G3).

## Done when
- [ ] MODELING.md §4.
- [ ] Every matching case passes, and the rest are pinned with reasons.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Interactive tools (024, 025).

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
