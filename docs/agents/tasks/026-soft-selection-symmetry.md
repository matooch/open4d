# 026 · Soft selection and symmetry modeling

Status: ready · Area: B · Depends on: 023 · Observations: G3 · Touches: ui, modeling

## Goal
Component transforms fall off smoothly around the selection, and edits mirror across a symmetry plane.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- GUI session G3 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- crates/ui/src/component_edit.rs (brief 06)

## Design decisions (planner)
- Soft selection: radius, falloff (linear, dome, bell, spline), surface vs volume distance. Weights feed `transform_points` as per-point blend factors.
- Symmetry: X, Y or Z plane in object space, with a tolerance. Each moved point moves its mirror partner.

## Done when
- [ ] EDITABLE.md §5.
- [ ] Tests: a move with radius 100 on a plane moves neighbours by the expected falloff fraction; symmetric moves stay symmetric.
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
