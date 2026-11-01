# 057 · Fields framework + Linear, Spherical, Box + Remapping

Status: done · Area: D · Depends on: 050 · Observations: O7, O11 · Touches: fields

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
- [x] FIELDS.md spec.
- [x] O7 and O11 field cases match.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `forge-fields` has the `Field` trait (`sample(p_world, &EvalCtx) -> FieldSample`, `sample_many`), `ShapeField` (shape + Remapping tab + inverse world matrix), `FieldList` (layers top to bottom, layer strength, Normal blend, empty list = value 1) and `field_from_node` / `FieldList::from_nodes`. Node types `forge.field.linear`, `forge.field.spherical`, `forge.field.box` (category Field, all parameters are descriptors) are registered by `FieldToolset`, installed in `forge_app::default_registry`. Tests: `crates/fields/tests/fields_parity.rs` (27/27 cases exact, tolerance 1e-5), `remap.rs` (5), `crates/app/tests/fields.rs` (5; object.create, param.set, undo, save/load). `KNOWN_MISMATCH` and unsupported lists are empty; the other 30 cases of `tests/golden/fields` are types and layers of 058/059. Deformer falloff is not wired (061).
- Rules derived (docs/specs/FIELDS.md): ramp `t`: Linear `0.5 + p_a/(2 length)` (`linear-defaults`, `-length`, `-direction-x`, `-rotated`), Spherical `1 - |p|/radius` (`spherical-radius`, `-scaled`, `-moved`), Box `1 - max |p_i|/size_i` (`box-defaults`, `box-size`); inner offset `t >= 1-i ? 1 : t/(1-i)` (`spherical-inner-0/-80`, `linear-inner-50`, `box-inner-0`); invert `1-v` (`spherical-remap-invert`); `min + v (max-min)` (`spherical-remap-minmax`); strength multiplies (`spherical-strength-50`, `layer-opacity-50` for the layer strength); step with 1 step maps 0.5 to 1 (`spherical-interp-step`), step/quantize 4 (`spherical-steps-4`, `-quantize-4`); Clip to Shape changes nothing on the grid (`linear-clip`, `spherical-clip`).
- Parity-pending (data cannot tell apart): order of invert, min/max and strength; Min/Max as remap or as clamp; where the clamps act; Step formula (`ceil` chosen) and whether Quantize differs (`floor(v n + 0.5)` chosen); Quadratic shape (symmetric ease, identity at curve 0 since `spherical-remap-contour` keeps 0.5) and the Curve parameter; Clip to Shape before/after remap; Remapping off; Mirror Graph; Inner Offset 100 %; zero sizes; Curve contour (Unsupported); colour and direction channels. Settling data are tabulated in FIELDS.md F5 (for example Spherical Inner Offset 0 % with Min 20 % / Max 80 %, or with Step 4, or with Invert plus Strength 50 %).
- Deviations: none from the design decisions. The Normal blend of a `FieldList` (lerp by layer strength) is minimal; 059 owns the other blend modes and checks `blend-normal`. `layer-opacity-50` is run here because it needs only layer strength.
- Files changed: crates/fields (Cargo.toml, src/{lib,node,remap,shape}.rs, tests/{fields_parity,remap}.rs), crates/app (Cargo.toml, src/lib.rs, tests/fields.rs), Cargo.lock, docs/specs/FIELDS.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, docs/roadmap/areas/D-deformers-fields.md, this brief.
- Open data: see FIELDS.md F5.
- Gate: `GATE: 783 tests passed, 0 failed`; `cleanroom: ok`; `ok: 471 cases`; `GATE: all passed`.
