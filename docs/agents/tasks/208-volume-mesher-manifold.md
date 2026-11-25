# 208 · Volume mesher: manifold output on thin features

Status: ready · Area: F · Depends on: 083, 206 · Observations: – (own design; reference behaviour on thin features not observed) · Touches: volume

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
- Result: one acceptance test added (`s8_lookdev_area_light_camera_dof_material_and_aovs` in `apps/forge/tests/render.rs`) plus the manual `docs/manual/rendering.md`. This brief has no observation, so parity is N/A. macOS is the observed OS; the existing CI matrix (macOS, Linux, Windows) runs the same test, and each OS render is compared with `tests/acceptance/s8-reference.png`.
- Rules derived (RENDER.md R19): the scene is built only through `forge new`/`forge cmd ... --save` (cube, Area light, physical camera, Standard material with noise on colour and fresnel on bump); the first camera and first light are used (R11, S5); `--dof` traces the thin lens, and changing only the focus distance changes the beauty; `--aov all` writes beauty plus ten passes; the same seed renders byte-identically; the object and material id passes hold background 0 plus a geometry id. The reference comparison requires each colour channel's mean within 1 dB and PSNR ≥ 30 dB.
- Deviations from the design decisions: the brief says "a node material with noise and fresnel", but the node-graph material's registry has only colour, scalar, texture, mix and Surface nodes, so the scenario uses the Standard material's shader node tree (noise + fresnel) instead. Adding noise/fresnel graph nodes would be a materials change outside this tests/docs brief. The comparison uses a checked-in macOS reference because cross-OS artifacts are not otherwise available inside one test run.
- Files changed: `apps/forge/tests/render.rs`, `tests/acceptance/s8-reference.png`, `docs/specs/RENDER.md` (R19), `docs/manual/rendering.md`, `docs/manual/SUMMARY.md`, `docs/roadmap/BACKLOG.md`.
- Open questions: none. Regenerate the reference with `FORGE_S8_WRITE_REFERENCE=1` only when the scene changes intentionally.
- The last lines of the gate output: `GATE: 2501 tests passed, 0 failed` and `GATE: all passed`.
