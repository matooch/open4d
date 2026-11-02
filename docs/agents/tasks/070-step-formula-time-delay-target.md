# 070 · Step, Formula, Time, Delay and Target effectors

Status: ready · Area: E · Depends on: 068, 052 · Observations: O14 · Touches: clone-engine

## Goal
Five deterministic effectors matching O14.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O14: [../../parity/effectors-2.md](../../parity/effectors-2.md) and its golden cases
- docs/parity/effectors-2.md (O14 cases)
- forge-expr (052) for Formula

## Design decisions (planner)
- Step: weight by index/(count-1) through a spline curve (default linear).
- Formula: the expression variables (id, count, time, …) as in the reference manual; evaluated with forge-expr.
- Time: parameter × time. Delay: per-clone smoothing over frames (blend/average/spring modes) — needs the previous frame state; keep it in the clone cache keyed by frame.
- Target: aim clones at a target object or the next clone.

## Done when
- [ ] O14 cases for these effectors pass at level B.
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
