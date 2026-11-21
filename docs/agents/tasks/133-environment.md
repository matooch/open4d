# 133 · Environment: sky, HDRI, background, floor

Status: done · Area: J · Depends on: 130, 137 · Observations: O29 · Touches: render-api, generators

## Goal
Sky and environment objects with HDRI lighting, background and floor objects.

## Inputs
- Area design: [../../roadmap/areas/J-materials.md](../../roadmap/areas/J-materials.md)
- O29: [../../parity/scene-defaults.md](../../parity/scene-defaults.md) and its golden cases
- docs/parity/scene-defaults.md

## Design decisions (planner)
- HDRI importance sampling in the CPU tracer.

## Done when
- [x] A scene lit by an HDRI renders without fireflies at 64 spp (PSNR vs 4096 spp ≥ 30 dB).
- [x] The gate passes (per-crate checks; the full gate once at the end of the session) and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests in render-cpu (`environment.rs`, 3: density and sampler consistency, furnace,
  64 spp vs 4096 spp PSNR 32.8 dB against the 30 dB goal), render-bridge (HDR file loaded into
  `Lighting::Image`, missing file warns) and the CLI (sky background and floor). No parity data:
  O29 observed no Environment, Sky or Floor objects, so these are our design (SCENE-OBJECTS.md
  S4); nothing is in KNOWN_MISMATCH or UNSUPPORTED.
- Rules: importance sampling by `luminance * sin(theta)` with a matching MIS density (RENDER.md
  R16); the uniform sampling scored 15.8 dB on the same scene (measured once by disabling the
  distribution). The object types, background flags and floor were done in brief 047.
- Deviations: the "HDRI importance sampling in the CPU tracer" is image-only; Sky and Constant
  keep uniform sampling. The colour of an Environment object does not tint its image.
- Files changed: crates/render-cpu (lights.rs `EnvDist`, integrator.rs, engine.rs,
  tests/environment.rs), crates/render-bridge (scene.rs, Cargo.toml, tests), apps/forge tests, docs.
- Open questions: observe the reference's Sky, Floor, Background and Environment objects
  (a form for them) so the parameter tables and the background behaviour can be checked.
