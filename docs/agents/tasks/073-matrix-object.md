# 073 · Matrix object

Status: waiting: O31 · Area: E · Depends on: 065 · Observations: O31 · Touches: clone-engine

## Goal
A Matrix object: a cloner that produces only matrices (no geometry), usable as a distribution source.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O31: [../../parity/clone-generators-2.md](../../parity/clone-generators-2.md) and its golden cases
- docs/parity/clone-generators-2.md (matrix cases)

## Design decisions (planner)
- Shares the cloner modes; produces `CloneData` with no prototypes. Exposes its matrices to Object-mode cloners and to the Tracer.
- Viewport: draw matrices as small axes (079).

## Done when
- [ ] O31 matrix cases pass.
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
