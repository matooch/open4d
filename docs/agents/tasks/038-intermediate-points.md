# 038 · Intermediate Points modes and R25 per-spline defaults

Status: ready · Area: C · Depends on: – · Observations: O17, O4 · Touches: generators, geometry

## Goal
Every spline supports None, Natural, Uniform, Adaptive and Subdivided sampling. Arc, Circle and Helix default to Uniform (8), as in R25. This removes brief 07's "R25 default not supported yet" rows.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O17: [../../parity/splines-2.md](../../parity/splines-2.md) and its golden cases
- O4 (see the area file)
- docs/parity/splines-2.md (defaults per spline)
- docs/specs/GENERATORS2.md §0a (brief 07 kept Adaptive for circle and helix)
- tests/defaults/r20.toml (the R20 overlay keeps Adaptive for R20 cases)

## Design decisions (planner)
- Shared module `crates/generators/src/spline_sampling.rs`: `sample(spline, mode, number, angle, max_length) -> Vec<DVec3>`. Uniform = equal parameter steps per segment (check against helix-uniform and circle-uniform).
- Switch the descriptor defaults (circle and helix intermediate = uniform), and make sure the R20 overlay still pins adaptive for the R20 cases.

## Done when
- [ ] GENERATORS2.md §0a updated.
- [ ] All existing spline-based cases still pass; the O17 uniform cases match.
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
