# 088 · Tracks for all value types + point-level animation

Status: waiting: O53 · Area: G · Depends on: 087 · Observations: O53 · Touches: core, anim

## Goal
Animate every parameter type (bool, int, enum, colour, vector, link where keyable) and points of editable meshes.

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)
- O53: [../../parity/animation-3.md](../../parity/animation-3.md) and its golden cases
- docs/parity/animation-3.md (O53)
- docs/specs/ANIM.md
- crates/core/src/track.rs

## Design decisions (planner)
- `KeyValue::Float` for numeric channels (vector/colour split into components); `KeyValue::Step(Value)` for bool/enum/link. Int: interpolate then round per O53 int-track rule.
- Point-level animation: a track per editable mesh holding whole point arrays per key; linear blend unless O53 shows otherwise.
- File format bump with a migration (163 pattern).

## Done when
- [ ] O53 cases (bool, enum, colour, int, pla, generator-param) pass at level B.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Euler/quaternion (096).

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
