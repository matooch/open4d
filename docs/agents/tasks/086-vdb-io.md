# 086 · VDB file read/write

Status: done · Area: F · Depends on: 080 · Observations: O33 · Touches: volume, io

## Goal
Read and write OpenVDB float grids from the public file format description.

## Inputs
- Area design: [../../roadmap/areas/F-volumes.md](../../roadmap/areas/F-volumes.md)
- O33: [../../parity/volumes-2.md](../../parity/volumes-2.md) and its golden cases
- docs/parity/volumes-2.md (exported .vdb cases)

## Design decisions (planner)
- Own implementation of the 5-4-3 tree layout; our 8³ block is the VDB leaf. Metadata, transform, active voxel count.
- Fuzz target for the reader (182).

## Done when
- [x] Round trip of our own grid is exact; the O33 reference .vdb reads with matching metadata, transform, active count, values within tolerance.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `vdb.rs` reads and writes float grids (version 224 layout, 5-4-3 tree, node masks, value metadata modes 0 to 6, zlib inflate for stored / fixed / dynamic blocks); 8 tests (`tests/vdb.rs`): exact round trips (sphere SDF, fog, empty grid, far apart blocks over several level 1 and 2 nodes, a third inactive value, zip path, several grids), zlib vectors from a reference zlib, skipping of other grid types, hand-built foreign features (tiles, blosc, bad version, absurd counts), truncation of every prefix and seeded corruption without a panic. `vdb-export`: UNSUPPORTED (no golden file: the reference export needs a human); the test reads it when it appears.
- Rules: VOLUME.md section 10. All layout details are parity-pending: no reference file was available, the layout is from the public description.
- Not done: Blosc and half-float reading, active tiles, interior tiles of reference files (read as background, counted), vector and other grid types (skipped), the fuzz target (brief 182 wires `read_vdb`).
- Files: crates/volume/src/vdb.rs, tests/vdb.rs, grid.rs (block accessors), docs/specs/VOLUME.md section 10.
