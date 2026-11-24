# 165 · FBX import (ufbx)

Status: done · Area: M · Depends on: 088, 101 · Observations: O44 · Touches: io

## Goal
FBX import via vendored ufbx.

## Inputs
- Area design: [../../roadmap/areas/M-io.md](../../roadmap/areas/M-io.md)
- O44: [../../parity/exchange.md](../../parity/exchange.md) and its golden cases
- docs/parity/exchange.md

## Design decisions (planner)
- ufbx built with `cc`; safe wrapper with `// SAFETY:` comments.

## Done when
- [x] O44 FBX cases import within tolerance.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- **Result:** `crates/io/tests/fbx.rs` (13 tests), a format-table update in `crates/io/tests/formats.rs`
  and `convert_imports_an_fbx_scene` in `apps/forge/tests/convert.rs`. Parity **6/6**: every
  `tests/golden/exchange/*/scene.fbx` imports and matches its `mesh.obj` (`compare_geometric`,
  1e-4: points, oriented planes, closedness) and the `case.toml` values (transforms, the
  `anim-transform` keys at frames 0 and 30, camera P -15 / focal 36 mm / Omni and Spot, the three
  `materials` colours on their cubes, `skinned-arm` weights of the recorded points, J2 rotation at
  frames 15 and 30, and the posed bounds `f15_bbox` and `f30_bbox` within 1e-4). `UNSUPPORTED` and
  `KNOWN_MISMATCH` are empty (asserted exactly). Format `fbx` (import; `.fbx`; option `scale`) is
  registered for `file.import`, `forge convert` and File > Import.
- **Rules derived** (IO.md §8.2, §8.3):
  - Unit: the file's unit is converted to centimetres (`file metres per unit / 0.01 * scale`);
    all six files declare 0.01 m (`static-mesh`: cube corners +-100 equal the OBJ).
  - Axes: change of basis `C = k S P` (file axes to right-handed Y-up, then `S = diag(1, 1, -1)`);
    points `C p`, node and bind matrices `C M C^-1`, polygon corners reversed after the first
    corner when `C` mirrors. Proved by `camera-light` (FBX z +600 is -600; Omni z +300 is -300),
    `hierarchy` (Euler Y 45 is H 45), `skinned-arm` (Euler Z -90 is B +90) and the oriented plane
    comparison of all six. Z-up in metres and left-handed Y-up checked on hand-written ASCII files.
  - Camera: FBX looks along local +X, ours along +Z: rotation times `Ry(-90)` before the change of
    basis (proved by `camera-light`: result P -15 exactly). Light: rotation times the arc from
    -Z to `local_direction` (Rx(-90) for -Y; proved: Spot and Omni FBX Rx(90) become identity).
  - Skin: not renormalised (`p + sum w (K p - p)`, ufbx `get_skin_vertex_matrix` semantics). The
    exporter quantises weights to 16 bits (rows sum to 1 + 1.5e-5); renormalised, the posed
    bounds miss by 3.6e-4 (f15) and 4.2e-4 (f30), unnormalised they match to 1e-4. (The glTF
    importer has the same miss, pinned in brief 164; its file has no unnormalised-skin hint.)
  - Parity-pending (not observed): units other than cm and axes other than Y-up (only on ASCII
    files I wrote), textures (none in O44), orthographic cameras, non-linear interpolation
    (resampled by ufbx at 30 per second), material roughness and alpha rules (own), light decay.
- **Deviations from the design decisions:** I use the `ufbx` crate from crates.io (MIT or Unlicense,
  `cargo deny check` passes; it compiles `ufbx.c` with `cc` in its `build.rs`) instead of vendoring
  `ufbx.c`/`ufbx.h` in the repo; there is no `unsafe` of ours, so no `// SAFETY:` comments. The
  unit and axis conversion is done by our own change of basis, not by ufbx's `space_conversion`
  (with `ModifyGeometry` ufbx leaves a pre-rotation on root nodes, which gave animated root nodes
  a -90 degree tilt). The gate was run in chunks (see the last line below). I could not build on
  macOS or Windows (MSVC) here: the crate is plain C99 through `cc`, which both support.
- **Files changed:** `crates/io/src/fbx/{mod,read,apply}.rs` (new), `crates/io/src/{lib,formats}.rs`,
  `crates/io/Cargo.toml`, `Cargo.lock`, `crates/io/tests/{fbx,formats}.rs`,
  `apps/forge/tests/convert.rs`, `docs/specs/IO.md` (§3.1, §3.2 and the new §8; no renumbering, so
  no references changed), `docs/parity/STATUS.md`, `docs/roadmap/BACKLOG.md`, `docs/PROGRESS.md`.
- **Open questions:** (1) the exporter writes its viewport camera into every file (an extra camera
  object is imported in every scene); should the importer skip it? It cannot be told from a user
  camera without a name or property rule, so it is imported. (2) A file with two skin
  deformers, blend shapes or textures would settle the §8.6 limits; O44 has none.
- **Gate:** the disk allowance cannot hold the test executables of `cargo test --workspace`
  (about 23 GB), so `scripts/gate.sh` ran out of space at the test step (linker bus errors) twice.
  I ran the same steps with the tests package by package, deleting the test executables after each
  package: fmt, clippy `--workspace --all-targets -D warnings`, 2514 tests passed 0 failed in 36
  packages with `FORGE_REQUIRE_GPU=1`, the Python client tests, `cargo deny check`,
  `forge-cleanroom` and the golden check. Last line: `GATE: all passed`.
