# 166 · FBX export (ASCII 7.4)

Status: done · Area: M · Depends on: 165 · Observations: O44 · Touches: io

## Goal
Our own ASCII FBX writer.

## Inputs
- Area design: [../../roadmap/areas/M-io.md](../../roadmap/areas/M-io.md)
- O44: [../../parity/exchange.md](../../parity/exchange.md) and its golden cases

## Design decisions (planner)
- Validate by re-importing with ufbx.

## Done when
- [x] Round trip export → import.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- **Result:** `crates/io/tests/fbx_export.rs` (13 tests) and two CLI tests in
  `apps/forge/tests/convert.rs`. The six O44 scenes go import (brief 165) -> export ASCII FBX 7.4 ->
  import and are compared object by object (names, parents, kinds, world matrices at 0, 0.25, 0.5,
  1 s within 1e-4, mesh points in order, faces exactly, UVs, materials, camera and light settings),
  against the reference OBJ with `compare_geometric`, and for `skinned-arm` against `f15_bbox` and
  `f30_bbox` within 1e-4 in centimetres and in metres; a second round changes nothing (1e-6).
  **6/6; no `UNSUPPORTED` or `KNOWN_MISMATCH`.** Scenes built in code: a hierarchy turned about three
  axes with a non-uniform scale (1e-9), linear, spline and step keys, quaternion rotation tracks,
  every camera and light setting (cm and m), materials with polygon selections and the default
  surface, a skin with J2 keyed (scale 1, 0.01 and 0.5), selected and detached roots, determinism,
  the file header, and `ufbx` reading the file as ASCII 7.4 without warnings. Format `fbx` now
  exports (`forge convert --format fbx`, `file.export`, option `scale`).
- **Rules derived** (IO.md §8.8; own design, no reference ASCII file, so all of it is
  parity-pending against the reference app; validated by re-import):
  - Everything is the inverse of the importer's §8.2 rules (reflection `S`, reversed winding,
    `D M D^-1`), `UnitScaleFactor = 1 / scale`.
  - Rotation = XYZ Euler degrees, `R = Rz Ry Rx` (`RotationOrder 0`), proved by the three-axis
    hierarchy round trip (1e-9); cameras and lights get the inverse frame change (`Ry(+90)`,
    `Rx(+90)`), proved by the camera and light round trip.
  - Skin: `Transform` is the mesh-to-bone matrix (the inverse of our bind matrix) and
    `TransformLink` the bone's world matrix. My first version wrote the mesh's world matrix in
    `Transform`; ufbx reads that field as `mesh_node_to_bone` and doubled the skin offset.
  - Rotations are sampled on every frame (also quaternion tracks): FBX interpolates Euler angles,
    so only per-frame samples reproduce a slerp (first version kept the two keys: wrong between).
  - Materials: Lambert for roughness 1, else Phong with `Shininess = 2 / roughness^2 - 2` (the
    inverse of the importer's rule); polygons without a material get an extra "Default" material.
  - Orthographic cameras: `OrthoZoom = width / 30` (ufbx's unit for foreign writers),
    parity-pending.
- **Deviations from the design decisions:** none for the writer (own code, no SDK, no copied
  template). Importer: the sensor width is now the film width (`film_size_inch`) instead of
  `aperture_size_inch`, which follows the render resolution; identical for all O44 files.
  The importer's `Error` gained `Empty` (maps to `NothingToExport`). The existing convert test that
  used "fbx" as an example of an unsupported export format now uses "dxf".
- **Files changed:** `crates/io/src/fbx/write.rs` (new), `crates/io/src/fbx/{mod,read}.rs`,
  `crates/io/src/formats.rs`, `crates/io/tests/fbx_export.rs` (new), `apps/forge/tests/convert.rs`,
  `docs/specs/IO.md` (§8.8, §3.1, §3.2, export frame), `docs/parity/STATUS.md`,
  `docs/roadmap/BACKLOG.md`, `docs/roadmap/areas/M-io.md`, `docs/PROGRESS.md`.
- **Open questions:** (1) does the reference app open and import our ASCII file with the same
  result (an observation with the file as input would settle it, and the ortho and material rules)?
  (2) A skinned generator object (a parametric cylinder with a skin) is not written: like the glTF
  writer it needs an editable polygon bind mesh (error "skin without polygon bind mesh").
- **Gate:** same chunked run as brief 165 (the disk cannot hold `cargo test --workspace`): fmt,
  clippy `--workspace --all-targets -D warnings`, 2528 tests passed 0 failed in 38 packages,
  Python client tests, `cargo deny check`, `forge-cleanroom`, golden check. Last line:
  `GATE: all passed`.
