# 167 · Alembic (Ogawa) import/export

Status: done · Area: M · Depends on: 088 · Observations: O44 · Touches: io

## Goal
Alembic Ogawa reader/writer for meshes, transforms and animation.

## Inputs
- Area design: [../../roadmap/areas/M-io.md](../../roadmap/areas/M-io.md)
- O44: [../../parity/exchange.md](../../parity/exchange.md) and its golden cases
- docs/parity/exchange.md

## Design decisions (planner)
- Own implementation from the public spec.

## Done when
- [x] O44 Alembic cases; round trip.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- **O44: 6/6.** Every `scene.abc` is imported through the format registry. The
  reference OBJ geometry matches at the manifest tolerance (geometric comparison,
  ignoring numbering). Hierarchy and frame-30 TRS match; both arm poses (f15 and
  f30) match the recorded world bounds at the unchanged tolerance. The camera
  archive restores the camera and has no mesh or light.
- **Round trips:** authored n-gons, UV corners, transforms, fixed-topology point
  animation, detached roots, scale, and static camera settings. The byte reader
  rejects truncated and unfrozen archives and invalid pointers.
- **Interoperability check:** public Alembic 1.8.9 `abcecho` and typed `IPolyMesh` /
  `IXform` readers opened the CLI's emitted arm `.abc`; 91 samples each have 106
  positions and 112 faces, f30 maximum X is 150, and J2 has the expected turn.
  The public C++ library is not a Forge3D dependency.
- **Scope:** Ogawa fixed topology, no HDF5, SubD schema, animated UVs or lens
  properties, skin semantics, materials or lights. Poses decompose to native
  TRS; shear or non-inherited transforms are not imported. Export samples authored
  tracks/generators rather than live app expressions or physics.
- **Files:** IO §6; `forge-io::abc` Ogawa reader/writer and import model,
  format registration and `crates/io/tests/alembic.rs`.
- **Gate:** `GATE: 2455 tests passed, 0 failed`; `GATE: all passed`.
