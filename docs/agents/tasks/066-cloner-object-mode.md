# 066 · Object-mode distributions with seeds and selections

Status: ready · Area: E · Depends on: 065, 027 · Observations: O21 · Touches: clone-engine

## Goal
Cloner Object mode: vertex, edge, polygon centre, surface, volume and axis distributions, with selection restriction and seeds.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O21: [../../parity/cloner-3.md](../../parity/cloner-3.md) and its golden cases
- docs/parity/cloner-3.md (O21 object-mode cases)
- tests/golden/cloner/ (object-* cases)
- brief 027 (selection tags) for restriction

## Design decisions (planner)
- Vertex/edge/polygon-centre are deterministic: match them exactly (positions, alignment to normals, up vector rule).
- Surface and volume distributions are random: try at most three hypotheses for the generator (IMPLEMENTER.md budget). If none matches, compare statistically (count exact, bbox, mean, variance within tolerance) and document it.
- Selection restriction reads the selection tag named in the cloner (027).

## Done when
- [ ] All deterministic object-mode cases match at level B.
- [ ] Random cases have a statistical test with the reason documented in MOGRAPH.md.
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
