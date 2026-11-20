# 123 · Material model: Standard material + material tag + IR compile

Status: done · Area: J · Depends on: – · Observations: O29 · Touches: material (new), plugin

## Goal
A Standard material node with the reference default channels, a material tag on objects, and a shading IR both renderers consume.

## Inputs
- Area design: [../../roadmap/areas/J-materials.md](../../roadmap/areas/J-materials.md)
- O29: [../../parity/scene-defaults.md](../../parity/scene-defaults.md) and its golden cases
- docs/parity/scene-defaults.md (O29 material defaults)

## Design decisions (planner)
- New crate `forge-material`. Channels as ParamDescs (colour, diffuse, luminance, transparency, reflectance layers, bump/normal, alpha) with O29 defaults.
- `compile(&Material) -> ShadingGraph` (small IR: constants, texture lookups, mix, BSDF lobes). CPU interprets it; GPU compiles to WGSL (142).
- Material tag: link + projection (projection details in 127).

## Done when
- [x] MATERIAL.md spec citing O29.
- [x] IR unit tests.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 8 tests in crates/material/tests/material.rs; no golden cases (O29 is a form only, O43 needs a renderer).
- Rules: O29 defaults pinned (`defaults_match_o29`); channel/mix/enable rules in MATERIAL.md M3 are parity-pending (published models).
- Deviations: materials and shaders cannot be scene nodes yet (`Document::create` refuses `Category::Material`; no Slot for them), so the tag's `material` link cannot be set and texture slots are shader-name strings; `Material::from_values` builds from a param map. A core brief must add hosting. Percent fields are stored as fractions.
- Files: crates/material (new), docs/specs/MATERIAL.md, crates/app (plugin install), icons material + tag_texture.
- Open: core slot for Material/Shader nodes.
