# 135 · Render API: snapshot, settings, outputs, AOV model, progress; harness compare_image

Status: done · Area: K · Depends on: 123 · Observations: – · Touches: render-api, parity-harness

## Goal
Stabilise `trait RenderEngine` with a scene snapshot, settings, outputs/AOVs and progress; add `compare_image` to the harness.

## Inputs
- Area design: [../../roadmap/areas/K-rendering.md](../../roadmap/areas/K-rendering.md)
- crates/render-api/src/lib.rs
- docs/ARCHITECTURE.md

## Design decisions (planner)
- Snapshot = flattened instances (mesh, matrix, material IR), lights, camera, environment. Engines never see the live document.
- `compare_image(expected, actual) -> {psnr, silhouette_iou}` with exposure normalisation.

## Done when
- [ ] RENDER.md spec; compare_image unit tests.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `forge-render-api` (snapshot, light/environment, camera, settings, outputs/AOVs, engine trait with progress events and cancel, frame ranges, path tokens, file writing incl. multipart EXR) and `forge_parity_harness::compare_image` (PSNR after exposure normalisation, silhouette IoU, silhouette mismatch within a pixel tolerance). 13 API tests + 8 compare_image tests, all pass. No golden cases at this brief (parity of renders is brief 136/137).
- Rules: docs/specs/RENDER.md R1-R5, R10, R12. Camera: positive pitch looks up (observed P -15 looks down, see 136 report).
- Deviations: none. Light/camera scene objects are not registered (no node types yet; follow-up).
- Files: crates/render-api/**, crates/parity-harness/src/image.rs, tests/image_compare.rs, docs/specs/RENDER.md.
- Open questions: none.
- Gate: run at the final commit of the track (see brief 139).
