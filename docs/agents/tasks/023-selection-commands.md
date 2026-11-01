# 023 · Selection commands: grow, shrink, convert, invert, connected, loop, ring

Status: ready · Area: B · Depends on: 011 · Observations: O26, G3 · Touches: geometry, ui

## Goal
The selection commands match the six observed O26 cases. Loop, ring and grow follow our own design until G3 is answered.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O26: [../../parity/selection-ops.md](../../parity/selection-ops.md) and its golden cases
- GUI session G3 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- tests/golden/selection-ops/ (each case has `selection.json`)
- crates/geometry/src/selection.rs (grow/shrink already exist from brief 05)

## Design decisions (planner)
- Commands `select.grow|shrink|invert|connected|convert {to, mode: tolerant|strict}|loop|ring`.
- Parity test: rebuild the start selection from `case.toml`, run the command, compare with `selection.json` exactly.
- Convert rules from O26: points → polygons strict (all corners), with tolerant as an option. A polygon needs all four of its edges (convert-edge-to-poly).
- Loop and ring use edge loops and rings on the half-edge mesh (quads: across the opposite edge; stop at poles and N-gons). Own design, parity-pending.

## Done when
- [ ] EDITABLE.md §4 (selection commands).
- [ ] All 6 O26 cases match.
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
