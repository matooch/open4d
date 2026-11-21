# 128 · Corner UVs on PolyMesh; primitive UV parity

Status: done · Area: J · Depends on: – · Observations: O28 · Touches: geometry, generators, io

## Goal
Every primitive emits UVs matching the reference (the golden `vt` data).

## Inputs
- Area design: [../../roadmap/areas/J-materials.md](../../roadmap/areas/J-materials.md)
- O28: [../../parity/uv-projection.md](../../parity/uv-projection.md) and its golden cases
- all primitive goldens (they already contain vt/f v/vt)
- docs/parity/uv-projection.md

## Design decisions (planner)
- `PolyMesh` gets a corner UV attribute (uvs + per-corner index). OBJ writer/reader handle vt. Comparison in the harness: UVs within 1e-5, matched per corner.

## Done when
- [x] Every primitive golden matches on UVs (or KNOWN_MISMATCH with reason).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 10/10 Part A primitives match per corner within 1e-5 (crates/generators/tests/uv_parity.rs); table sizes equal for 9 (capsule counted over used entries: the reference lists one unused entry); cube-segments pinned 29 vs 32 (corner values all match, the reference repeats shared values).
- Rules: cell pattern u/v per quad, planar caps with mirrored bottom, sphere v = polar angle, capsule v by chord length, tube inner wall flipped, torus cell based (UV.md U2).
- Deviation: UVs live in `forge_geometry::CornerUvs` beside the mesh, not inside `PolyMesh` (200+ struct literals across other tracks). OBJ vt reader/writer in forge-io not done (follow-up).
- Files: crates/geometry/src/uv.rs, crates/generators/src/uv.rs, tests/uv_parity.rs, docs/specs/UV.md.
