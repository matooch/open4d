# 197 · Shading IR to WGSL codegen and shader cache

Status: waiting: 123, 125, 126 · Area: R · Depends on: 123, 125, 126 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: material, shaders

## Goal
Compile the shading IR (crates/material, brief 123) and the shader nodes (125, 126) to WGSL: one specialised shader per material, cached by IR hash, recompiled on edit. This is the bridge from node materials to every GPU tier.

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Done when
- [ ] Every IR op and every shader node from 125/126 has a WGSL equivalent; nodes that cannot be expressed return a documented fallback.
- [ ] A test harness evaluates each IR/shader on a grid of sample points both through the CPU interpreter and through the WGSL code on the GPU (skipped without an adapter) and compares within 1e-4 (f32).
- [ ] Recompiling a 50-node material takes under 100 ms (measured in a test, not a hard CI limit).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
