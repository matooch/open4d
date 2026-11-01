# 083 · Volume Mesher (dual contouring, adaptivity, threshold, normals)

Status: waiting: O32 · Area: F · Depends on: 081 · Observations: O32 · Touches: volume, geometry

## Goal
Turn a volume into a polygon mesh like the reference Volume Mesher; shared iso-surface module (also used by Metaball if useful).

## Inputs
- Area design: [../../roadmap/areas/F-volumes.md](../../roadmap/areas/F-volumes.md)
- O32: [../../parity/volumes.md](../../parity/volumes.md) and its golden cases
- docs/parity/volumes.md (mesh cases, "mostly quads" note)

## Design decisions (planner)
- Dual contouring over the active band, QEF via SVD (nalgebra), quad output; marching cubes fallback behind a flag.
- Adaptivity: quadric-error edge collapse on the result, weighted by curvature.
- Normals from the SDF gradient.

## Done when
- [ ] All O32 mesher cases pass compare_volume_mesh; polygon count within 10 % at adaptivity 0.
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
