# 139 · Render settings, image sequences, formats, multipass/AOVs

Status: done · Area: K · Depends on: 136, 130 · Observations: O29 · Touches: render-api, render-cpu

## Goal
Render settings object (size, frame range, output path with tokens, format) and AOVs.

## Inputs
- Area design: [../../roadmap/areas/K-rendering.md](../../roadmap/areas/K-rendering.md)
- O29: [../../parity/scene-defaults.md](../../parity/scene-defaults.md) and its golden cases
- docs/parity/scene-defaults.md (render settings)

## Design decisions (planner)
- AOVs: beauty, albedo, normal, depth, object id, per-light; EXR multipart output.

## Done when
- [ ] CLI renders an image sequence; AOVs written.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: output settings object (`OutputSettings`: size, frame range current/all/manual, step, rate, path template with `{frame}`, `{frame:N}`, `{pass}`, `{name}`, format png/tif/jpg/hdr/exr, 8/16/32 bit, alpha, straight alpha, separate or multipart), frame list, path expansion, file writing (invalid format/depth pairings and multipart without exr are errors); AOVs (depth, normal, albedo, position, object id, material id, shadow, diffuse, specular, emission) in `forge-render-cpu`; scene adapter `forge-render-bridge` (triangulation, crease-angle normals, framing camera); `forge render` CLI renders one frame or an image sequence with passes, separate or as a multipart EXR. Tests: 13 api, 3 bridge, 4 CLI render tests, S1 PNG render acceptance (`s1_png_render`, un-ignored).
- Rules: RENDER.md R3, R12. O29 defaults (320 x 240, 30 fps, current frame, step 1, 8 bit, no alpha, `Name0000.TIF`) are the `OutputSettings` defaults. No golden cases belong to this brief (O29 recorded no files).
- Deviations: the scene has no camera or light objects, so the CLI frames the geometry itself and uses the default light; render settings are not stored in the scene file yet (command-line flags instead; follow-up with the document-hosted render settings). Layered PSD, stereo, regions from the CLI, fields and the render queue are not done.
- Files: crates/render-api/src/files.rs, crates/render-bridge/**, apps/forge/src/render.rs, apps/forge/tests/render.rs, docs/specs/CLI.md, docs/manual/reference/cli.md.
- Open questions: where the render settings live in the document (core agent), light and camera node types and their registration (icon table).
