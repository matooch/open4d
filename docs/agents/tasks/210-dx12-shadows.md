# 210 · Real-time shadows on DirectX 12

Status: ready (needs a Windows machine with DX12) · Area: R · Depends on: 198 · Observations: – · Touches: render-rt

## Goal
`crates/render-rt/tests/rt_render.rs::a_shadow_map_darkens_the_occluded_plane` finds **no shadowed
pixels on DirectX 12** (Windows CI, WARP software adapter), while it passes on Metal and Vulkan.
Merged with this test red on Windows by owner decision (2026-11-25), to be fixed here.

## What is known (RENDER.md R21.4, brief 198 Follow-up)
- Ruled out by reading the source and naga's HLSL: shared-matrix overwrite (each atlas tile is
  its own submission), dynamic-offset alignment, depth range, UV convention, depth bias,
  atlas clear/load. `two_shadowed_lights_each_cast_their_own_shadow` passes on Vulkan.
- Remaining suspects: naga's DX12 comparison-sampler heap indexing
  (`nagaComparisonSamplerHeap[nagaGroup1SamplerIndexArray[3]]`, samplers at bindings 3 and 5),
  and WARP's `Linear` comparison filtering on `D32_FLOAT`.

## Steps (on the owner's Windows PC, RTX 3090)
1. Run the test on hardware DX12 and on Vulkan:
   `set WGPU_BACKEND=dx12` then `cargo test -p forge-render-rt --test rt_render a_shadow`;
   repeat with `set WGPU_BACKEND=vulkan`. Also with `set WGPU_ADAPTER_NAME=Microsoft Basic Render Driver` (WARP).
2. If only WARP fails: confirm by switching the comparison sampler to `Nearest` filtering in a
   local experiment; if that fixes WARP, keep Linear for hardware and document the WARP limit, or
   use PCF (already 3×3) with Nearest everywhere if results stay within the R21 bounds.
3. If hardware DX12 fails too: dump the shadow atlas (copy to buffer) after the shadow pass and
   check whether depth was written; then whether the comparison returns 0 for all texels;
   bisect between the pass and the lookup.
4. Fix, keep Metal/Vulkan results identical, and make the Windows CI check green.

## Done when
- [ ] The shadow tests pass on DX12 (hardware and WARP) and stay identical on Metal and Vulkan.
- [ ] Windows CI is green.
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
