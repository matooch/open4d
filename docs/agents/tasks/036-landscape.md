# 036 · Landscape primitive

Status: ready · Area: C · Depends on: 063 · Observations: O18 · Touches: generators, noise

## Goal
Landscape with every observed parameter. The mesh is deterministic per seed. Comparison is statistical unless the exact noise rule is found.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O18: [../../parity/primitives-3.md](../../parity/primitives-3.md) and its golden cases
- docs/parity/primitives-3.md (Landscape notes: deterministic from its settings; size, furrows, sea/plateau levels, multifractal, borders, spherical)

## Design decisions (planner)
- Grid (width × depth segments) with heights from fractal noise (`forge-noise`), with sea and plateau clamping and borders at sea level.
- Parity: the grid topology and x/z positions match exactly. The heights are compared by histogram and by correlation with the golden. Exact heights are parity-pending.

## Done when
- [ ] PRIMITIVES3.md §Landscape.
- [ ] Tests: topology exact; height statistics within tolerance for all 4 cases.
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
