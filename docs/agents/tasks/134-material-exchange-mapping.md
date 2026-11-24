# 134 · Material exchange mapping (glTF PBR to node material)

Status: done · Area: J · Depends on: 124, 164 · Observations: – · Touches: io, material

## Goal
Map glTF metallic-roughness materials to and from our materials.

## Inputs
- Area design: [../../roadmap/areas/J-materials.md](../../roadmap/areas/J-materials.md)

## Design decisions (planner)
- Documented mapping table in MATERIAL.md.

## Done when
- [x] Round-trip test glTF → ours → glTF keeps parameters.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- glTF core materials import as hosted node graphs. GLB exports from evaluated
  Surface inputs, preserving RGBA, fractional metallic/roughness, emission, alpha
  mode/cutoff, sidedness and all five core texture slots with their scalar inputs.
- The new round trip checks exact representable factors, shared embedded image
  deduplication, PNG buffer views, an opaque external URI, and equal reopened graphs.
  All existing glTF tests pass, including O44 geometry, material/UV and skin checks.
- The native shader still approximates fractional metal with its existing flag;
  alpha/sidedness, occlusion and packed maps are exchange-only. Additional UV sets,
  sampler overrides and glTF material extensions are not supported.
- Files: MATERIAL §M9, IO §5, material Surface values, glTF import/export,
  embedded preview image loading, and `crates/app/tests/gltf_exchange.rs`.
- Gate: `GATE: 2493 tests passed, 0 failed`; `GATE: all passed`.
