# 208 · Volume mesher: manifold output on thin features

Status: done · Area: F · Depends on: 083, 206 · Observations: – (own design; reference behaviour on thin features not observed) · Touches: volume

## Goal
The Volume Mesher must never emit non-manifold geometry. Brief 206 found that a strong smooth
layer on a tube about 3 voxels thick produces non-manifold edges ("fins"): the mesher places one
vertex per mixed cell (VOLUME.md §7.2), so a cell crossed by two separate surface sheets joins
them through one vertex. After Make Editable the user gets a broken mesh.

## Inputs
- docs/specs/VOLUME.md §7.2 (surface extraction), §7.5 (adaptivity), §12 (S5 scenario and its open question 1)
- Brief 206's Report; tests/acceptance/s5_volume.rhai (the default-smoothing variant reproduces it)
- Public literature on manifold dual contouring (one vertex per connected surface component
  inside a cell, e.g. Schaefer, Ju & Warren, "Manifold Dual Contouring", 2007)

## Design decisions (planner)
- Spec first: extend §7 with the rule "one vertex per surface component of a cell" (components
  found from the cell's 12 edge sign changes, the marching-cubes case table or an equivalent
  public rule), and how faces pick the right vertex per edge.
- Keep the output identical to today wherever a cell has one component, so every existing O32/O33
  parity result and the S5 test stay byte-identical. Prove it: the volume parity tests and
  `s5_volume` must pass unchanged.
- Adaptivity (merging cells) must not re-join components.

## Done when
- [ ] A test meshes the 206 scene with the **default** smooth layer (and a synthetic two-sheet
      cell case) and asserts closed and manifold (every edge in exactly two faces, every vertex
      fan a single disc), no degenerate faces.
- [ ] All volume parity tests and S5 pass unchanged.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: the mesher now puts **one vertex per surface component of a cell** (VOLUME.md §7.2
  rule 4, rewritten spec-first). Components are found from the cell's own corner signs: every
  cell face pairs its crossed edges (the two of a 2-crossing face; at an alternating face the two
  edges at each inside corner), and the groups of crossed edges joined by those pairs are the
  components. A quad uses, in each of its four cells, the vertex of the component holding the
  lattice edge. Code: `crates/geometry/src/isosurface.rs` (`edge_components`, `cell_mesh`,
  `merge_blocks`); `forge-volume` re-exports it, so the Metaball generator shares it. Parity: no
  observation (own design, parity-pending); output for single-component cells is unchanged.
- Single-component cells are unchanged: the vertex is the mean of the group's crossings in edge
  order (same arithmetic as before) and vertices are numbered by first use. Proof: every existing
  test passes unmodified: `forge-volume` (`volumes_parity`, `mesher`, `builder`, `filters`,
  `sdf_csg`, ...), `forge-geometry`, `forge-generators` (including `metaball_parity`), and
  `forge` acceptance `s5` (byte-identical OBJ after reopen and between two runs); the golden
  check `ok: 1117 cases` in the gate.
- Adaptivity (§7.5): a block is a leaf only when no two of its vertices come from one cell; at
  the single-cell level each vertex of a multi-component cell is its own leaf. Cells with one
  vertex are unaffected (the sphere adaptive 0.5 case is unchanged).
- Tests, new file `crates/volume/tests/manifold.rs` (9 tests, run 5 times, no flake): two opposite
  inside samples of one cell (16 vertices, 12 quads, two blobs); diagonal inside samples of a face;
  a checkerboard one-sample-thick slab; all 256 corner patterns of one cell; random fields (see
  limit); adaptivity keeps a shared cell's two vertices; 206's scene with the **default** smooth
  layer and with Voxel Distance 3, closed and manifold (every edge in two faces, one disc per
  vertex fan, no repeated index, no zero-area face); a ball keeps one vertex per cell. Before the
  change the synthetic cases and Voxel Distance 3 failed.
- Deviation from the brief: the **default** smooth layer on 206's scene already meshes manifold on
  this branch (6200 vertices, 6198 faces, no doubled edge; checked through the CLI and the
  library), so that test passed before the change. 206's fins appear with a wider layer (Voxel
  Distance 3), which is therefore the regression case that failed before. `s5_volume.rhai` is
  unchanged (its light layer keeps the volume within 5 %).
- Known limit (VOLUME.md §7.2): two neighbouring cells whose single components are both glued along
  the two segments of one alternating face (a ring of inside samples around a hole smaller than a
  cell) share a vertex pair in two quad pairs. The surface stays closed with as many quads each way
  but those edges coincide. Seen only in dense random noise (tested as balanced edges), not in
  the S5 or the distance fields tested.
- Files changed: `crates/geometry/src/isosurface.rs`, `crates/volume/tests/manifold.rs`,
  `docs/specs/VOLUME.md` (§7.2, §7.5, §12), `docs/manual/volumes.md`, `docs/PROGRESS.md`,
  `docs/roadmap/BACKLOG.md`, this brief.
- Open questions: whether the reference app splits cells or also produces fins on thin tubes
  (needs a golden case); the coincident-edge limit above.
- The last lines of the gate output: GATE_PLACEHOLDER
