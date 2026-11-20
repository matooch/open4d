# 080 · forge-volume: sparse grid, mesh/spline/points to SDF, CSG; harness compare_volume_mesh

Status: done · Area: F · Depends on: – · Observations: O32 · Touches: volume (new), parity-harness

## Goal
A new `forge-volume` crate with a sparse grid, conversions to signed distance fields, and CSG; plus a geometric mesh comparison in the harness.

## Inputs
- Area design: [../../roadmap/areas/F-volumes.md](../../roadmap/areas/F-volumes.md)
- O32: [../../parity/volumes.md](../../parity/volumes.md) and its golden cases
- docs/parity/volumes.md (O32)
- crates/geometry/src/boolean.rs (winding number)

## Design decisions (planner)
- `SparseGrid<T>`: HashMap<IVec3 block key, Box<[T; 512]>> with background value and voxel transform; active-voxel iteration; get/set; trilinear sample.
- Mesh to SDF: BVH over triangles, exact unsigned distance within the band, sign from the generalized winding number, then fast-sweep fill out to the band width.
- Spline to SDF: capsule chain. Points to SDF: union of spheres (optional smooth-min).
- CSG: min/max/negate, polynomial smooth-min.
- Harness: `compare_volume_mesh(expected, actual, voxel)` → Hausdorff (≤ voxel/2), area and volume within 2 %, equal Euler characteristic. Unit-test it on spheres.

## Done when
- [x] VOLUME.md spec citing O32.
- [x] Unit tests: sphere SDF error < 0.1 voxel; cube CSG.
- [x] compare_volume_mesh tested.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
The builder node (081) and the mesher (083).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 9 grid / SDF / CSG tests in `crates/volume/tests` (sphere SDF error below 0.1 voxel at voxel 10 and 5, band and interior fill, surface samples exactly `+0.0`, cube CSG incl. unstored interior, spline capsules, point spheres, smooth union), 6 `compare_volume_mesh` tests in `crates/parity-harness/tests/volume_compare.rs`. No golden parity cases belong to this brief (they are 083).
- Rules derived (from O32 cube-defaults, checked vertex by vertex in brief 083): samples at multiples of the voxel size; a sample exactly on the surface reads `+0.0` and is outside; band 2 voxels each side.
- Deviations: BVH with winding-number dipoles for the sign, per-voxel exact distance instead of per-triangle splat plus fast sweep (the band is exact, so no sweep is needed); interior fill by x-row spans instead of a fast sweep (VOLUME.md section 2.5).
- Files: crates/volume (new), crates/parity-harness/src/volume.rs, src/lib.rs, tests/volume_compare.rs, docs/specs/VOLUME.md, BACKLOG row, this brief.
- Open questions: none for 080.
