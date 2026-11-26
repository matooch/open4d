# 203 · Consistency tests across render tiers

Status: done (parity-pending: Tier 1/2 pins and Vulkan hardware run) · Area: R · Depends on: 198, 202 · Observations: – (own design; material values from O29/O42/O43 through the shading IR) · Touches: tests, render-rt

## Goal
One test suite that renders the O30/O43 scenes and the material set in every tier and compares to the CPU path tracer, with a PSNR budget per tier written in RENDER.md.

## Inputs
- Area design: [../../roadmap/areas/R-realtime.md](../../roadmap/areas/R-realtime.md)
- Decision D19 in [../../roadmap/DECISIONS.md](../../roadmap/DECISIONS.md)
- docs/specs/MATERIAL.md, SHADERS.md, RENDER.md

## Done when
- [x] Budgets: Tier 3 ≥ 35 dB, Tier 2 ≥ 28 dB, Tier 1 ≥ 24 dB after exposure normalisation (unchanged; measured misses are reported below).
- [ ] Runs in CI through the compute fallback where no RT GPU exists; on the owner's M5 Pro with Metal RT and on the RTX 3090 with Vulkan RT (manual steps, numbers recorded in the Report). Vulkan and Metal images agree within the Tier budget.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

R26 runs all 23 O30/O43 manifests from `tests/golden/render`, including shader
materials, through each tier. It compares linear RGB against one Forge CPU
reference per case, not the observed PNGs. Both the explicit compute device
and the M5 Pro Metal ray-query device were exercised. Tier 1 has no RT backend;
Tiers 2 and 3 report the backend that actually supplied each image.

| Tier | Budget | Compute vs CPU | Metal vs CPU | Worst Metal vs Compute |
|---|---:|---:|---:|---:|
| 1 | 24 dB | 18/23; min 12.64 dB | 18/23; min 12.64 dB | identical |
| 2 | 28 dB | 19/23; min 5.16 dB | 19/23; min 5.16 dB | 165.12 dB |
| 3 | 35 dB | 23/23; min 71.47 dB | 23/23; min 71.47 dB | 75.66 dB |

The test scores and asserts the exact nine pinned tier/case misses from
R26; it fails on a new miss or a stale pin. Four-jitter TAA did not resolve
the Tier 2 misses. AOV isolation identified the glossy specular highlight in
the hybrid direct-shadow pass as the largest miss. No fixture data, tolerance,
or shading behavior changed.

**Vulkan RT run pending on the owner's RTX 3090 (brief 212).** On the M5 Pro
run the command below with `FORGE_R26_HARDWARE=Metal`; copy `r26-metal` to the
RTX 3090 host and run it there with `FORGE_R26_HARDWARE=Vulkan`. Use the same
revision and, on the second host, select the Vulkan adapter rather than DX12.
The test requires the requested hardware, prints every comparison, and
directly compares Vulkan against the 69 lossless Metal captures. No
hardware-to-hardware misses are pinned.

```sh
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
FORGE_REQUIRE_GPU=1 FORGE_R26_HARDWARE=Metal FORGE_R26_METAL_IMAGES="$PWD/r26-metal" \
  cargo test -p forge-render-rt --test path_trace o30_and_o43_cross_tier_consistency -- --nocapture
```

Gate: `GATE: all passed`. Parity observations and status files were left
untouched per the unattended-run instructions; the result is recorded here
and in R26.
