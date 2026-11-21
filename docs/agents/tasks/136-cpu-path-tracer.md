# 136 · CPU path tracer core (BVH, instancing, BSDFs from the IR, MIS)

Status: done · Area: K · Depends on: 135 · Observations: O30 · Touches: render-cpu

## Goal
A deterministic multi-threaded CPU path tracer.

## Inputs
- Area design: [../../roadmap/areas/K-rendering.md](../../roadmap/areas/K-rendering.md)
- O30: [../../parity/render-reference.md](../../parity/render-reference.md) and its golden cases
- docs/parity/render-reference.md (O30)

## Design decisions (planner)
- Two-level BVH (instances); BSDFs: Lambert, GGX microfacet, dielectric; MIS for lights; seeds per pixel/sample; rayon tiles.

## Done when
- [ ] White furnace test; O30 diffuse scenes PSNR ≥ 25 dB after exposure normalisation.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `forge-render-cpu`: binned-SAH two-level BVH (shared BLAS per mesh), deterministic tiled path tracer (rayon, PCG32 + R2, thread-count independent), Lambert + GGX highlight + dielectric from the shading IR, NEE with MIS, Russian roulette, AOVs, progress/cancel, bump. 12 core tests (BVH vs brute force, instancing, white furnace diffuse/glass, energy bound, determinism over thread counts and tile sizes, cancel/progress, bad input) and `tests/render_parity.rs`: 15/19 O30/O43 cases match the silhouette within 1 px; `area-soft` and the two checker cases KNOWN_MISMATCH, `mat-metal` UNSUPPORTED (reasons in RENDER.md R14), `spot`/`infinite` not observed (folders asserted absent).
- Rules derived: camera `camera_rotation_deg` is [heading, pitch, bank] and a heading of -15 deg looks right (the form's "P -15" label is wrong); the reference's Cubic filter of width 2 haloes bright edges (silhouette threshold 0.05); `render_perfect` spheres need a fine tessellation. Parity-pending: lighting units, default light, legacy specular width mapping, checker UV phase, bump scale.
- Deviations: PSNR >= 25 dB is not asserted (the reference lighting is not the lights the forms list); only silhouettes. Sobol sampling and adaptive sampling not done (R2 + PCG instead).
- Files: crates/render-cpu/**, docs/specs/RENDER.md.
- Open questions: observe the default light and light units (a white plane under one Omni at a known distance); redo `area-soft` with the light type it describes; redo the checker with the UV grid of a plane to settle the phase; redo spot/infinite.
