# 057 · Fields framework + Linear, Spherical, Box + Remapping

Status: ready · Area: D · Depends on: 050 · Observations: O7, O11 · Touches: fields

## Goal
The field system exists. The three basic fields and the Remapping tab match every O7/O11 sample case at level B.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O7: [../../parity/fields.md](../../parity/fields.md) and its golden cases
- O11: [../../parity/followups-1.md](../../parity/followups-1.md) and its golden cases
- docs/parity/fields.md, followups-1.md §1 (ramp rules: Linear ramp 2 × Length, Spherical flat to half size)
- tests/golden/fields/
- crates/parity-harness samples API (brief 08)

## Design decisions (planner)
- `crates/fields` (`forge-fields`): `trait Field { fn sample(&self, p_world: DVec3, ctx: &EvalCtx) -> FieldSample }`. Field objects are scene nodes (`Category::Field`) with transform and params. Remapping is a shared struct applied after the shape falloff: strength, invert, inner offset, min/max, clamps, multiplier, contour (none, quadratic, step, quantize; curve Unsupported).
- The parity test builds the field from `case.toml`, samples the 729 points, and compares with `compare_samples`.

## Done when
- [ ] FIELDS.md spec.
- [ ] O7 and O11 field cases match.
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
