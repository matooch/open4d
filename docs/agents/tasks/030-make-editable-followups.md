# 030 · Make Editable follow-ups from O3

Status: done · Area: B · Depends on: 003, 028 · Observations: O3 · Touches: generators, ui

## Goal
Make Editable and Current State to Object behave like R25 in the O3 points where brief 03 guessed differently.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O3: [../../parity/make-editable.md](../../parity/make-editable.md) and its golden cases
- docs/parity/make-editable.md (cases and behaviour notes)
- docs/specs/EDITABLE.md §1 (brief 03 parity-pending rules)

## Design decisions (planner)
- Extrude (and other cap-making generators): Make Editable gives the side walls as the object, with `Cap 1` and `Cap 2` as polygon-object children carrying polygon selection tags C1/C2 (O3 extrude).
- Current State to Object keeps the **original** selected (O3 behaviour notes); the new object is named like the source, with no suffix.
- A UVW tag is added on conversion (once J/128 gives meshes UVs; until then, record it as parity-pending).
- Spline primitives → editable spline (028).
- Generators with consumed inputs: one object named after the generator (O3 subdivision).

## Done when
- [x] EDITABLE.md §1 updated, with the parity-pending markers removed where O3 proves the rule.
- [x] Tests for each O3 case, including the meshes in tests/golden/make-editable/.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `crates/generators/tests/make_editable_parity.rs` discovers the 9 cases. Meshes 8/8 (cube, cube-deformed, sphere-moved, subdivision strict at 1e-4; cube-deformed-csto, subdivision-csto, extrude, array geometric as their forms say); observed trees 8/8 asserted (all but `array`, plus the `spline` tree which has no mesh). No `UNSUPPORTED`, no `KNOWN_MISMATCH`. Before this brief the cases had no test; the unchanged code already gave the cube, cube-deformed, both csto, sphere-moved, subdivision meshes, array mesh and the spline; `extrude` (caps as children) and the tag rules were the gaps.
- Rules derived: deformers stay live, points unbent (`cube-deformed`); csto: same-named copy right after the original, deformers baked, original left selected (`cube-deformed-csto`, `subdivision-csto`); consumed input gives one object named after the generator and lends its tags, so the Phong tag of `subdivision` comes from the cube (`subdivision`); cap-making generators: side walls as the object, children `Cap 1` / `Cap 2` (identity transform, only their own points, selection tag `C1` / `C2` over all polygons) (`extrude`). Make Editable leaves the converted object selected, caps not selected; csto leaves the original (selection rule from the O3 behaviour notes, `crates/app/tests/make_editable_commands.rs`).
- Parity-pending: `array` tree (the export names eight objects `Cube.8` ... `Cube.1`, parametric cubes; we make one merged polygon object, so only the surface matches); UVW tag and the two data tags (no UVs, storage only); tag order on caps (observed UVW, selection, Phong; we give selection, Phong); cap polygon start corner (R25 start cap `[3, 2, 1, 0]`, our R20-style corner differs, hidden by the geometric compare); lathe/loft/sweep caps (same code path, no case); dedupe rule for tags lent by consumed inputs; behaviour with several objects selected; csto of a cap-making generator (we give one object with the caps in its mesh). Follow-up cases to observe: Array Make Editable tree (names, types, transforms of the children, tags); Extrude with Cap off or Caps = Triangles; Lathe with caps; csto on Extrude; a Cloner Make Editable; both commands with two objects selected; a tag listing of the extrude cap (UVW and Phong order); UVW for a cube after J/128.
- Deviations: none from the design decisions; the UVW tag is not added (decision allows parity-pending until meshes have UVs). Added a tag type `forge.polygon_selection` for the `C1` / `C2` tags (no selection tag existed).
- Files changed: docs/specs/EDITABLE.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief; crates/generators/src/{polygon,spline_gens,lib}.rs, crates/command/src/builtin/mesh.rs; tests crates/generators/tests/make_editable_parity.rs, crates/app/tests/make_editable_commands.rs.
- Gate: `GATE: all passed` (716 golden cases valid).
