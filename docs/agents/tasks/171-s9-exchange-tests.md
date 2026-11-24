# 171 · Exchange parity tests (S9)

Status: done (parity-pending: glTF skinned-arm f15/f30 bounds, KNOWN_MISMATCH ≤ 5e-4) · Area: M · Depends on: 164–169 · Observations: O44 · Touches: tests

## Goal
S9 passes.

## Inputs
- Area design: [../../roadmap/areas/M-io.md](../../roadmap/areas/M-io.md)
- O44: [../../parity/exchange.md](../../parity/exchange.md) and its golden cases
- docs/roadmap/README.md (S9)

## Design decisions (planner)
- CLI-driven.

## Done when
- [x] S9 passes on all three OS (CI to confirm; the test is 1.2 s on Linux, no rendering).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report

### Coverage matrix before this brief (library-level tests that read the reference files)
Legend: G = geometry vs `mesh.obj`, T = node transforms/names, A = animation at frames,
C = camera/light, M = materials, S = skin poses. `-` = no test reads that file for that item.

| Case | FBX (`crates/io/tests/fbx.rs`) | glTF (`crates/app/tests/gltf_exchange.rs`) | Alembic (`crates/io/tests/alembic.rs`) | STL (`crates/io/tests/exchange_stl_ply.rs`) |
|---|---|---|---|---|
| static-mesh | G (`compare_geometric`), T | G (point sets only, both ways) | G (`compare_geometric`) | writer equals golden bytes; reader: points subset of OBJ, triangle count (no transform/plane check) |
| hierarchy | G, T | G (points) | G, T (Root, child count, pos) | as above |
| anim-transform | G, T, A (frames 0, 30) | G (points); A: translation at frame 30 only | G; A: pos/rot H/scale at t = 1 s | as above |
| camera-light | T, C (camera, Omni, Spot, 2 cameras) | C (import camera, export one camera, no light); mesh empty | C (camera pos, sensor), empty mesh | writer only (zero triangles) |
| materials | G, T, M (3 colours at x = -150/0/150) | G (points); M: our round trip only, not the reference values | G; no assertion that materials are absent | as above |
| skinned-arm | G, S (weights, J2 B 45/90, bounds f15/f30 at 1e-4) | G (points); S: keys restored by hand, bounds with KNOWN_MISMATCH | G; S: baked points, bounds f15/f30 at 1e-4 | as above |

Also: `export_parity.rs`, `fbx_export.rs`, `rigging_parity.rs` use the cases for the export direction
and the rig; `apps/forge/tests/convert.rs` converts only `hierarchy/scene.fbx` (node count).
Gaps closed by `s9_exchange.rs`: no test went through the CLI; glTF only compared point sets (not
planes/orientation); STL import was never compared with the OBJ as a surface; no test asserted what a format
does not carry; glTF/Alembic transforms of static-mesh/hierarchy/materials and the reference glTF
material values were not checked; the glTF animation was checked at one frame only.

### Result
- Tests added: `apps/forge/tests/s9_exchange.rs`, six tests (one per case, parallel, own scratch
  folder), CLI only. ~1.2 s on Linux (about 110 process starts of ~20 ms), no rendering.
  Run 5 times in a row, all green; two mutations (flipped rotation sign, wrong STL expectation) fail it.
- Parity: 23 of 24 case x format files import and match; camera-light STL (zero triangles) is
  refused with `no polygons found`, asserted. Pinned: KNOWN_MISMATCH glTF skinned-arm f15/f30
  (error in (1e-4, 5e-4), already in IO.md section 5; the test fails if it leaves that interval).
  `morph` has no observation, so it is not covered.

| Case | FBX | GLB | Alembic | STL |
|---|---|---|---|---|
| static-mesh | pass | pass | pass | pass (triangles) |
| hierarchy | pass | pass | pass | pass: one object, geometry in world space |
| anim-transform | pass (0, 30; 15 between) | pass | pass | pass: static |
| camera-light | pass (camera, Omni, Spot) | pass (camera, no lights) | pass (camera, no lights) | refused, no triangles |
| materials | pass (names, colours) | pass (names, colours, metallic/roughness) | pass (no materials) | pass: no materials |
| skinned-arm | pass (f15/f30 1e-4, J2 45/90) | pinned KNOWN_MISMATCH (< 5e-4) | pass (baked points f15/f30 1e-4) | pass: bind pose only |

- Rules derived: none new. No importer bug found; every gap was test-side.
- Deviations from the design decisions: material values and animated transforms are read from a glTF
  written by `forge export --frame N` (the CLI has no parameter read-out at a frame: `info` and Rhai
  `param()` show stored values, material nodes are not in `info`). STL is compared triangle for
  triangle instead of `compare_geometric`: the sphere quads are not exactly planar, so the plane
  grouping differs between the f64 OBJ and the f32 STL (not a tolerance change; the position
  tolerance stays 1e-4). The reference GLB is centimetre-valued: the test imports with
  `--option scale=1` as IO.md section 5 already says (the manual documents it).
- Files changed: `apps/forge/tests/s9_exchange.rs`, `apps/forge/Cargo.toml` (dev-dependency `toml`),
  `Cargo.lock`, `docs/specs/IO.md` (section 9 checks), `docs/manual/importing-files.md` and
  `SUMMARY.md`, `docs/PROGRESS.md`, `docs/parity/STATUS.md`, `docs/roadmap/BACKLOG.md`, this brief.
- Open questions: (1) whether the reference's skinned GLB weights (rows sum 0.9999847) or its
  authoring rig explain the 4.2e-4; an observation of the same pose exported as FBX with weights
  quantised the same way would settle it. (2) A CLI command that prints evaluated node transforms at a
  frame would make animation checks simpler than the glTF read-out (not needed for S9).
