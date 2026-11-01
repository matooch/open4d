# 078 · Clone selection and weight tags, clone cache

Status: waiting: O50 · Area: E · Depends on: 065, 027 · Observations: O50 · Touches: clone-engine

## Goal
Tags that restrict effectors to selected clones, store per-clone weights, and bake clone data.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O50: [../../parity/clone-toolset-3.md](../../parity/clone-toolset-3.md) and its golden cases
- docs/parity/clone-toolset-3.md

## Design decisions (planner)
- Clone selection tag: a set of clone indices; effectors with that selection only act on those.
- Weight tag: per-index weights feeding CloneData.weights.
- Cache: bake CloneData per frame into the document (binary blob in the native file, 163).

## Done when
- [ ] O50 tag cases pass; a baked cache plays back identically.
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
