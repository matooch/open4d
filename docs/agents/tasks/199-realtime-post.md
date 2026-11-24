# 199 · Post effects and viewport Render mode

Status: waiting: 198 · Area: R · Depends on: 198 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: render-rt, viewport, ui

## Goal
Screen-space AO and reflections, bloom, temporal anti-aliasing and filmic tone mapping; a viewport display mode "Render" that uses render-rt.

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Done when
- [ ] Each effect has a toggle in the render settings and a test that it changes the image as intended (e.g. AO darkens a crease).
- [ ] Switching the viewport to Render mode and back works with undo-free UI state (egui_kittest).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
