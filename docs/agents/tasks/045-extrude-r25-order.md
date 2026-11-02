# 045 · Extrude R25 vertex order

Status: ready · Area: C · Depends on: 044 · Observations: O1 · Touches: generators

## Goal
The R25 extrude cap start corner (brief 07's KNOWN_MISMATCH `r25-drift/extrude-rectangle-caps-quadrangles`) matches without breaking the R20 cases.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O1 (see the area file)
- docs/specs/GENERATORS2.md §11.1 (three hypotheses from brief 07)
- the AppVersion mechanism from brief 07 (harness)

## Design decisions (planner)
- The cap corner order differs between R20 and R25. Our generators target R25 (DECISIONS D12), so switch to the R25 order. The R20 extrude cases then need an app-version-aware expectation: either a cap-rotation-insensitive comparison for R20 cases, or a documented `KNOWN_MISMATCH "R20 order"` list. Choose the one that keeps the most exact checks.

## Done when
- [ ] GENERATORS2.md §11.1 resolved; drift test 7/7.
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
