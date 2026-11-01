# 139 · Render settings, image sequences, formats, multipass/AOVs

Status: waiting: O29 · Area: K · Depends on: 136, 130 · Observations: O29 · Touches: render-api, render-cpu

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
- Result: tests added; parity N/M (`UNSUPPORTED` / `KNOWN_MISMATCH` with reasons).
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations from the design decisions, and why.
- Files changed.
- Open questions or data that would settle them (the reviewer turns these into follow-up forms).
- The last lines of the gate output.
