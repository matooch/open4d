# 075 · Clone-toolset Text

Status: waiting: O31, O48 · Area: E · Depends on: 039, 065 · Observations: O31, O48 · Touches: clone-engine, generators

## Goal
Text as clones: letters, words and lines as clone groups, so effectors act on them.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O31: [../../parity/clone-generators-2.md](../../parity/clone-generators-2.md) and its golden cases
- O48: [../../parity/text.md](../../parity/text.md) and its golden cases
- docs/parity/text.md (O48)
- docs/parity/clone-generators-2.md
- brief 039 (text spline)

## Design decisions (planner)
- Reuse the text spline and extrude from 039; group the resulting pieces per letter/word/line as clones with matrices at their origin (rule from O31).
- Fonts: bundled open font (never the reference fonts); parity at level C on the outline, exact on clone counts and order.

## Done when
- [ ] Clone counts, order and centres match O31/O48 within tolerance.
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
