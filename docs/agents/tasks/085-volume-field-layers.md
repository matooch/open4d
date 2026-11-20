# 085 · Field layers inside the volume builder

Status: done · Area: F · Depends on: 081, 059 · Observations: O33 · Touches: volume, fields

## Goal
Fields as builder entries (adding density/distance from a field).

## Inputs
- Area design: [../../roadmap/areas/F-volumes.md](../../roadmap/areas/F-volumes.md)
- O33: [../../parity/volumes-2.md](../../parity/volumes-2.md) and its golden cases
- docs/parity/volumes-2.md (field cases)

## Design decisions (planner)
- Sample the field at voxel centres within the active band; mode per entry (add/subtract/multiply).

## Done when
- [x] O33 field cases pass.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: field nodes as builder children act as layers (VOLUME.md section 9); `field-layer` matches geometrically (2648 points as the reference, Hausdorff 1.0; nearest-vertex distance 0.54); 2 builder tests. Whole volumes golden set: 20 of 21 geometric, `smooth-strength` pinned.
- Rules derived: sampled at the active voxels' world positions, `t = max(0, (f - 0.5) / 0.5)`, add: `value - K t` with `K = 0.625 voxel` (fit 6.2 to 6.25 at voxel 10); subtract and multiply are own design (rows: subtract, intersect). All parity-pending (one case, default mode).
- Data note: the O33 table says the field-layer mesh has 2168 points; the golden mesh has 2648.
- Files: crates/volume/src/{filters,node}.rs, tests/builder.rs, tests/volumes_parity.rs, Cargo.toml (forge-fields), docs/specs/VOLUME.md section 9.
- Open questions: field layers on fog volumes; the per-entry mode names of the reference; other fields.
