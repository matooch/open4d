# 061 · Deformer falloff through field lists (all deformers)

Status: done · Area: D · Depends on: 057 · Observations: O24 · Touches: modifiers, fields

## Goal
Every deformer takes a field list. An empty list means full effect. The O24 cases match.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O24: [../../parity/deformer-falloff.md](../../parity/deformer-falloff.md) and its golden cases
- docs/parity/deformer-falloff.md (the field is in the deformer's list; the empty list gives full effect)

## Design decisions (planner)
- The deformer result is blended per point: p' = p + w·(deform(p) − p), with w = the field value at the point's world position.

## Done when
- [x] MODIFIERS.md §falloff; all 9 O24 cases match.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added (`crates/modifiers/tests/falloff.rs`: every deformer has the `fields` list, weight blend on Spherify, a deleted field link is skipped; the 9 O24 cases join `deformers_parity.rs`). Parity 9/9 O24 at level A, plus the existing 55 (64/64); `UNSUPPORTED` and `KNOWN_MISMATCH` stay empty.
- Rules derived (MODIFIERS.md §10): `p' = p + w·(deform(p) − p)`, `w` = `FieldList::sample` at the world position of the point *before* the deformer (the object's world matrix applied), not clamped; empty list = full effect (`bend-empty-list`); layer strength scales `w` (`bend-field-strength-50`); the field's remapping drives `w` (`bend-spherical` vs `bend-spherical-inner`); the rule holds for Bend, Twist, Taper, Bulge (`bend-*`, `twist-linear`, `taper-spherical`, `bulge-linear`) and is implemented generically for Shear and Spherify (no O24 case; Spherify unit-tested). Parity-pending: the deformed object off the origin (world vs object-local sampling), weights outside 0..1 (Strength 200 %, clamp off), Shear/Spherify with a field, rows beyond one layer (059).
- Deviations: the deformer's list is `fields` (a `LinkList` of Field nodes, as the Plain effector's). Layer strength has no per-row storage in a link list, so it is a new field-node param `layer_strength` (percent, default 100 %) read by `FieldList::from_nodes` (so the effector falloff reads it too). Real list rows (one field in several lists) come with 059.
- Files changed: crates/modifiers/{Cargo.toml,src/lib.rs,tests/deformers_parity.rs,tests/falloff.rs}, crates/fields/src/{lib.rs,node.rs}, docs/specs/{MODIFIERS.md,FIELDS.md}, docs/manual/reference (regenerated), docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, docs/roadmap/areas/D-deformers-fields.md, this brief.
- Open data that would settle pending rules: Bend 90 degrees with the cube at (0, 100, 0) and the O24 Spherical field; Bend with the field's Strength 200 % and Clamp Max off; Shear and Spherify with a Spherical field in the list; a field placed as a child of the deformer.
- Gate (last lines): `GATE: 798 tests passed, 0 failed` ... `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`.
