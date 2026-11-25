# 211 · Metal hardware ray-tracing backend (part 2 of 200)

Status: ready · Area: R · Depends on: 200 · Observations: – (own design) · Touches: render-rt (or render-rt-native)

## Goal
Implement `RtBackend` (R23, brief 200) with **Metal ray tracing** (acceleration structures and
intersection queries): through wgpu's experimental ray-query support on Metal if it covers
the R23 trait, otherwise per D19 with a narrow `objc2-metal` backend behind the same trait.

## Done when
- [ ] On the Mac (M5 Pro), Metal RT hits equal the compute backend's on the R23 test scene
      (same ids; t and barycentrics within 1e-4); skipped without Metal RT.
- [ ] Feature detection picks Metal RT where available and falls back to compute otherwise.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Inputs
- Brief 200 (RtBackend trait, compute backend, test scene) and RENDER.md R23; decision D19
- docs/roadmap/areas/R-realtime.md (reference machines: M5 Pro = Metal, RTX 3090 = Vulkan)

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
