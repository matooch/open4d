# 203 · Consistency tests across render tiers

Status: waiting: 198, 202 · Area: R · Depends on: 198, 202 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: tests, render-rt

## Goal
One test suite that renders the O30/O43 scenes and the material set in every tier and compares to the CPU path tracer, with a PSNR budget per tier written in RENDER.md.

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Done when
- [ ] Budgets: Tier 3 ≥ 35 dB, Tier 2 ≥ 28 dB, Tier 1 ≥ 24 dB after exposure normalisation (adjust only with a written reason).
- [ ] Runs in CI through the compute fallback where no RT GPU exists; on the owner's M5 Pro with Metal RT and on the RTX 3090 with Vulkan RT (manual steps, numbers recorded in the Report). Vulkan and Metal images agree within the Tier budget.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
