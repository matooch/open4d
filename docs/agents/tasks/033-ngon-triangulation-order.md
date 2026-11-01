# 033 · N-gon handling: reference triangulation order and edge visibility

Status: ready · Area: B · Depends on: – · Observations: O6, O18 · Touches: geometry, generators, viewport

## Goal
N-gons are triangulated the way the reference exports them where the goldens show it (dodecahedron 12 triangles + 12 quads, the cut triangles of slices), and hidden N-gon edges are not drawn.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O6: [../../parity/modeling-tools.md](../../parity/modeling-tools.md) and its golden cases
- O18: [../../parity/primitives-3.md](../../parity/primitives-3.md) and its golden cases
- tests/golden/platonic/type-dodeca (O18 note: each pentagon is a triangle plus a quad)
- the slice cases pinned as geometric-only (cone/torus/capsule slice)
- docs/specs/GEOMETRY.md §6

## Design decisions (planner)
- Derive the exporter's N-gon split rule from the dodecahedron and the slice cut faces. If one rule explains them, switch those slice cases from geometric to exact.
- Viewport: draw only the original N-gon boundary edges, not the internal triangulation edges.

## Done when
- [ ] GEOMETRY.md §6 updated.
- [ ] The dodecahedron matches exactly; the slice cases are upgraded or keep a documented reason.
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
