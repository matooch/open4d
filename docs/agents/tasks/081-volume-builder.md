# 081 · Volume Builder node (object list, modes, SDF/fog, voxel size, hierarchy)

Status: done · Area: F · Depends on: 080 · Observations: O32, O33 · Touches: volume, plugin

## Goal
The Volume Builder generator: an ordered object list with union/subtract/intersect per entry, SDF or fog type, voxel size and hierarchy handling.

## Inputs
- Area design: [../../roadmap/areas/F-volumes.md](../../roadmap/areas/F-volumes.md)
- O32: [../../parity/volumes.md](../../parity/volumes.md) and its golden cases
- O33: [../../parity/volumes-2.md](../../parity/volumes-2.md) and its golden cases
- docs/parity/volumes.md, volumes-2.md

## Design decisions (planner)
- Node with a list param (object link + mode + enabled + per-entry settings), evaluated top to bottom like the reference list.
- Fog type: density grid (inside = 1, fall off at the band); SDF type uses 080.
- Output is a volume, not a mesh; the mesher (083) consumes it. Until 083 exists, tests compare voxel counts/values on the grid.

## Done when
- [x] O32/O33 builder cases pass through the mesher once 083 lands; grid-level unit tests now.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `forge.volume_builder` and `forge.volume_mesher` (plugin `VolumePlugin`, mesher evaluator), `build_volume`, `mesh_volume_node`; 12 grid-level tests (`tests/builder.rs`) and the golden test `tests/volumes_parity.rs`: 12 of 21 cases match geometrically (`compare_volume_mesh` at voxel size), 9 of them also with equal point and face counts and every vertex within 1e-3 (cube x3, sphere x2, union-two, hierarchy, platonic of `torus-defaults`, mesher-threshold). Unmatched: 7 filter cases (brief 082), `field-layer` (085), `mesher-adaptive` (083, `KNOWN_MISMATCH`). subtract / intersect have no golden data (mode not scriptable in the reference): unit-tested only, parity-pending.
- Rules derived: the mesher rule of VOLUME.md section 7.2 (cube-defaults, exact); spline tube radius 0.735 range voxel (spline-source, spline-radius; parity-pending, one range); fog density `clamp((-sdf - 0.7 voxel)/(4 voxel))` (fog-input: geometric match, 1568 vs 1664 points; parity-pending).
- Data notes: `torus-defaults` holds a Platonic (type 4, icosa), not a torus (the case file says so); the observation text counts 2168 points for `field-layer` but the golden mesh has 2648.
- Deviations: per-entry mode is an `objects` LinkRows override list (the reference list UI is not scriptable); inputs are the builder's children; mesher and builder types and evaluator live in `forge-volume` (no edit to forge-generators). Not wired into the app registry (the host adds `VolumePlugin`; that is outside this brief).
- Files: crates/volume/src/{node,mesher}.rs, tests/{builder,volumes_parity}.rs, Cargo.toml, docs/specs/VOLUME.md sections 6, 7.
- Open questions: per-entry modes; spline radius at other ranges; the reference's fog density; the surface threshold beyond 0.5.
