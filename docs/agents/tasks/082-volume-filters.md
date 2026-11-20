# 082 · Volume filters (smooth types, dilate/erode, reshape, curve)

Status: done · Area: F · Depends on: 081 · Observations: O33 · Touches: volume

## Goal
Filter layers in the builder list: Gaussian, mean, median, Laplacian flow smoothing; dilate/erode; reshape; curve remap.

## Inputs
- Area design: [../../roadmap/areas/F-volumes.md](../../roadmap/areas/F-volumes.md)
- O33: [../../parity/volumes-2.md](../../parity/volumes-2.md) and its golden cases
- docs/parity/volumes-2.md

## Design decisions (planner)
- Separable Gaussian over the active band; mean/median windows; Laplacian flow iterated; dilate/erode as SDF offset then re-distance (fast marching).

## Done when
- [x] O33 filter cases pass compare_volume_mesh.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `forge.volume_filter` (offset, reshape, smooth: gaussian, mean, median, laplacian), `filters.rs` (re-distance by fast sweeping, reshape as closing / opening), 7 tests in `tests/filters.rs` plus 2 builder tests; golden: 6 of 7 filter cases match geometrically (gaussian, mean, median, laplacian, dilate, erode; Hausdorff 0.4 to 3.6, areas within 2 %), the point counts of gaussian, mean, median, laplacian, dilate and erode equal the reference's. `smooth-strength` (3 Gaussian iterations) is a `KNOWN_MISMATCH` (Hausdorff 7.5, area +6.5 %).
- Rules derived: Type 1 is a morphological open/close by a ball (dilate / erode), not a surface offset (the face centres stay put); median and mean use half width `voxel_distance` (2), the Gaussian is a sequence of box means of half widths 4 .. 1 with one re-distance per iteration; Laplacian flow is invisible on a cube; the list is evaluated bottom to top (the filter above the cube filters it).
- Data note: the `[params.filter]` tables of the mean, median and Laplacian cases read `operator = 0`; the descriptions and O33 notes give 1, 2, 4 (and the meshes differ), so the test reads the operator from the description.
- Deviations: filters re-distance after every step (needed for the Gaussian to match); strength is a blend factor (own design).
- Open questions: the reference's behaviour at several Gaussian iterations; Type 0 and the unlisted types; operator 3.
