# 072 · Effector falloff via field lists

Status: done · Area: E · Depends on: 068, 059 · Observations: O22 · Touches: clone-engine, fields

## Goal
Every effector takes a field list as falloff (same field stack as deformers).

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O22: [../../parity/effectors-3.md](../../parity/effectors-3.md) and its golden cases
- docs/parity/effectors-3.md (falloff cases)
- brief 059 (field list layers)

## Design decisions (planner)
- Evaluate the field list at each clone position (world space) and multiply the effector strength. Reuse `forge-fields` exactly; no copy of the layer logic.

## Done when
- [x] Falloff cases pass.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

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
- Result: falloff via field lists was already wired for every effector (shared `FieldList::from_param`, plain-field-* cases 14/14 exact). Added spec section "Field lists of any shape" (MOGRAPH.md E8) and the test `effector_falloff_takes_a_whole_field_list` (Spherical under Invert, disabled row, Random effector).
- Parity-pending: multi-layer lists on an effector (no observation).
- Files: docs/specs/MOGRAPH.md, crates/app/tests/cloner.rs, BACKLOG, this brief.
