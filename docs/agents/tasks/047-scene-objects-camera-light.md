# 047 · Camera, Light, Environment, Floor and Sky as scene objects

Status: waiting: O29 · Area: C · Depends on: – · Observations: O29 · Touches: generators, core, viewport

## Goal
These scene objects exist as nodes with R25 parameters and defaults, show in the viewport, and are ready for the renderers.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O29: [../../parity/scene-defaults.md](../../parity/scene-defaults.md) and its golden cases
- docs/parity/scene-defaults.md §2–3

## Design decisions (planner)
- Node types `forge.camera` (focal, sensor, FOV, projection, film offset, clipping, DOF params), `forge.light` (type, colour, intensity, falloff, cone angles, area shape and size, shadow type), `forge.environment`, `forge.floor`, `forge.sky`.
- The viewport draws gizmos (camera frustum, light shapes). "Look through camera" sets the viewport camera.

## Done when
- [ ] SCENE-OBJECTS.md spec with tables citing O29.
- [ ] Tests: descriptor defaults equal O29; the viewport camera from a camera object gives the same projection matrix as the formula.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Rendering (K).

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
