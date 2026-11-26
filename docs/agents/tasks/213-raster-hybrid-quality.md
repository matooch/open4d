# 213 · Close the Tier 1/2 quality gaps (pinned in R26)

Status: ready · Area: R · Depends on: 201, 203 · Observations: – (own design) · Touches: render-rt

## Goal
Brief 203's cross-tier check pins nine below-budget comparisons (RENDER.md R26). Most are a
few tenths of a dB short and come from raster approximations, but **`mat-glossy` Tier 2 scores
5.16 dB** (budget 28) and Tier 1 12.62 dB (budget 24): the narrow direct specular highlight
differs, and "hybrid shadows alone reproduce the full hybrid miss", which points at a bug in
how the hybrid ray-traced shadow term is applied to specular (e.g. shadowing the highlight
with the wrong light sample, or multiplying specular by a diffuse-only visibility).

## Steps
1. `mat-glossy` first: render Tier 1, Tier 2 with only RT shadows, and the CPU reference;
   compare the specular-only and diffuse-only AOVs (R3) to locate the difference; fix the
   cause in the spec (R24/R21) and code. Target: both tiers within budget, unpin.
2. `mat-bump-noise` Tier 1: the raster normal AOV scores 14.52 dB vs CPU; find why the bump
   normal differs from the CPU's R20.1b/R21.3 rule and fix.
3. `cube-plane`, `omni`, `omni-shadow` Tiers 1/2 (0.1–1.2 dB short): only if a spec-level
   cause is found (shadow bias, ambient term); otherwise leave pinned with the evidence.
Each fix must keep Tier 3 and the hardware-vs-compute comparisons unchanged; runs on lavapipe
(Linux) and Metal.

## Done when
- [ ] `mat-glossy` Tiers 1 and 2 meet the R26 budgets (pins removed).
- [ ] Every other pin either removed or re-justified in R26 with new evidence.
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
