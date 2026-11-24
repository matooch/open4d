# 204 · Real-time performance targets

Status: waiting: 199, 201 · Area: R · Depends on: 199, 201 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: render-rt, bench

## Goal
Benchmarks for the S10 scene (1 M polygons, 10 000 clones): Tier 1 at ≥ 60 fps and Tier 2 at ≥ 30 fps at 1080p on the reference GPU; frame-time breakdown per pass.

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Done when
- [ ] Benchmarks exist and report per-pass timings; the owner runs them on the reference machine (manual step) and records the numbers in the brief.
- [ ] No pass regresses by more than 10 % between runs on the same machine (bench check).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
