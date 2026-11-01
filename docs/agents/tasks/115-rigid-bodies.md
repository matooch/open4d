# 115 · rapier integration: rigid body tag, shapes, scene settings, per-frame cache, scrubbing; compare_trajectory

Status: waiting: O39 · Area: I · Depends on: 050, 010 · Observations: O39 · Touches: dynamics (new), parity-harness

## Goal
Rigid bodies through rapier3d with deterministic cached simulation and level-C parity against O39.

## Inputs
- Area design: [../../roadmap/areas/I-dynamics.md](../../roadmap/areas/I-dynamics.md)
- O39: [../../parity/dynamics.md](../../parity/dynamics.md) and its golden cases
- docs/parity/dynamics.md (O39)

## Design decisions (planner)
- New crate `forge-dynamics`. Fixed step 1/(fps × substeps); state cached per frame; scrubbing back resimulates from the nearest cached frame.
- Shapes: auto (convex hull), box, sphere, static mesh. Units: cm in the scene, metres in rapier – convert at the boundary.
- Harness `compare_trajectory` with per-case thresholds from case.toml `[tolerance]` (rest position, first contact ±1 frame, bounce ratio ±10 %, slide distance ±10 %).

## Done when
- [ ] O39 cases meet thresholds; two runs give identical caches.
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
