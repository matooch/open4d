# 163 · Native format: migrations, binary encoding, external references

Status: ready · Area: M · Depends on: – · Observations: – · Touches: core, io

## Goal
A versioned native file with migrations, a compact binary encoding for large arrays, and external file references.

## Inputs
- Area design: [../../roadmap/areas/M-io.md](../../roadmap/areas/M-io.md)
- crates/core/src/file.rs

## Design decisions (planner)
- Header with format version; migration functions per version step; meshes/caches as binary chunks; relative paths for references.

## Done when
- [ ] Old test files load through migrations; fuzz target added (182).
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
