# 137 · Lights (omni, spot, infinite, area, sky) and shadows

Status: done · Area: K · Depends on: 136 · Observations: O30, O29 · Touches: render-cpu, generators

## Goal
Light objects with reference defaults and falloff modes.

## Inputs
- Area design: [../../roadmap/areas/K-rendering.md](../../roadmap/areas/K-rendering.md)
- O30: [../../parity/render-reference.md](../../parity/render-reference.md) and its golden cases
- O29: [../../parity/scene-defaults.md](../../parity/scene-defaults.md) and its golden cases
- docs/parity/scene-defaults.md (light defaults)
- render-reference.md

## Design decisions (planner)
- Falloff modes (none, inverse square, linear) per O29; area light shapes rectangle/disc/sphere.

## Done when
- [ ] O30 light cases meet PSNR targets.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: omni, spot (smooth cone), infinite (with angle), area (rectangle, disc, sphere; one-sided or two-sided; visible or not), sky/constant/image environments, falloff none/linear/inverse linear/inverse square, hard and soft shadows, density and colour, diffuse/specular switches, default head light, shadow AOV. 17 tests in crates/render-cpu/tests/lights.rs, including closed forms (disc and sphere area light irradiance, cosine law, falloffs, soft penumbra).
- Matched/pinned: the light cases of O30 (omni, omni-shadow) match geometry (shadows are not compared: the reference lighting is parity-pending); spot and infinite were never observed (black), area-soft is pinned (see RENDER.md R14).
- Deviations: light node types are not registered in the scene (no icon table edits; follow-up with the scene adapter); falloff Steps and object-shaped area lights return Unsupported.
- Files: crates/render-cpu/src/lights.rs, tests/lights.rs, docs/specs/RENDER.md R8.
