# 083 · Volume Mesher (dual contouring, adaptivity, threshold, normals)

Status: done · Area: F · Depends on: 081 · Observations: O32 · Touches: volume, geometry

## Goal
Turn a volume into a polygon mesh like the reference Volume Mesher; shared iso-surface module (also used by Metaball if useful).

## Inputs
- Area design: [../../roadmap/areas/F-volumes.md](../../roadmap/areas/F-volumes.md)
- O32: [../../parity/volumes.md](../../parity/volumes.md) and its golden cases
- docs/parity/volumes.md (mesh cases, "mostly quads" note)

## Design decisions (planner)
- Dual contouring over the active band, QEF via SVD (nalgebra), quad output; marching cubes fallback behind a flag.
- Adaptivity: quadric-error edge collapse on the result, weighted by curvature.
- Normals from the SDF gradient.

## Done when
- [x] All O32 mesher cases pass compare_volume_mesh; polygon count within 10 % at adaptivity 0.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: all 10 O32 mesher cases and the O33 meshes of polygon, spline and fog sources pass `compare_volume_mesh`; 10 polygon cases match the reference exactly in point and face count with every vertex within 1e-3 (including mesher-adaptive: 53 points, 72 faces, 30 quads). Unit tests in `tests/mesher.rs` (outward orientation, closedness, threshold dead zone and shift, adaptivity monotone and edge-preserving, gradient normals, absolute iso).
- Rules derived: one vertex per mixed cell (mean of edge crossings) and a quad per crossed edge, a sample on the surface is outside (cube-defaults); adaptivity is an octree merge of aligned 4 and 8 cell blocks with the block mean as vertex (mesher-adaptive: all 53 vertices are block means), plane tolerance `2 * adaptive * voxel` (one case, borderline 9.97 vs 10.04: parity-pending). Not dual contouring with QEF: the reference output is exactly the mean-of-crossings cell vertex.
- Deviations from the design: no QEF/SVD, no quadric collapse (the reference structure is simpler and matches); marching cubes fallback not built.
- Files: crates/volume/src/mesher.rs, tests/mesher.rs, docs/specs/VOLUME.md section 7.
- Open questions: adaptive tolerance at other values; the surface threshold beyond 0.5; absolute iso; what Voxel Range Threshold does.
