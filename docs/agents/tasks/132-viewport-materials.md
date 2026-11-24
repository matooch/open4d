# 132 · Viewport material display (PBR preview, textures)

Status: done · Area: J · Depends on: 123, 143 · Observations: – · Touches: viewport

## Goal
The viewport shows material colour, textures and a PBR approximation.

## Inputs
- Area design: [../../roadmap/areas/J-materials.md](../../roadmap/areas/J-materials.md)

## Design decisions (planner)
- Preview approximation from the IR (base colour, roughness, metallic, normal map).

## Done when
- [x] Baseline image test.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Native preview: assigned Standard materials feed the material IR and hosted shaders
  into a bounded UV bake. The GPU uses the base colour, diffuse roughness, metal flag,
  and tangent-space normal map. The editor and headless viewport screenshots share
  the same upload path. A cloner's own material is multiplied by clone tint.
- Baseline: the GPU test compares 36 interior chart pixels with a fixed four-colour
  image, then checks clone tint, roughness, metal and normal-map response. The editor
  test covers assignment, a material edit, tag removal and reuse of an unrelated draw.
- Scope: this is a native preview, not reference parity. It uses one unrestricted
  texture tag per draw; layered/restricted tags, inherited prototype materials,
  camera mapping, object/world normal maps and non-UV shader evaluation are outside
  the preview. LOD proxies are bypassed on tagged draws to preserve corner UVs.
- Files: VIEWPORT §15, viewport `material_preview` and draw shaders, and the UI
  viewport upload/cache adapter and screenshot path.
- Gate: `GATE: 2485 tests passed, 0 failed`; `GATE: all passed`.
