# 074 · Fracture and Voronoi Fracture

Status: waiting: O31 · Area: E · Depends on: 065, 033 · Observations: O31 · Touches: clone-engine, geometry

## Goal
Fracture (explode segments, connected, sorted) and Voronoi Fracture (point sources, seeds, inner faces, offset).

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O31: [../../parity/clone-generators-2.md](../../parity/clone-generators-2.md) and its golden cases
- docs/parity/clone-generators-2.md (fracture cases)
- forge-geometry::boolean (winding numbers, robust predicates)

## Design decisions (planner)
- Fracture: split children by connectivity (polygon islands) into clones; ordering rules from O31.
- Voronoi: cells by clipping the mesh by bisector planes of the seed points, inner faces capped and triangulated (033 rules). Point sources: distribution source points (count, seed).
- Parity: geometric (Hausdorff, total volume, cell count); exact positions only if seeds reproduce.

## Done when
- [ ] Fracture cases match; Voronoi cell counts and total volume within 1 %.
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
