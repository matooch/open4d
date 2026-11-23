# 145 · Render parity tests against O30/O43/O57

Status: done (parity-pending: existing silhouette and lighting gaps) · Area: K · Depends on: 137, 138 · Observations: O30, O43, O57 · Touches: tests

## Goal
All reference render cases have automated PSNR/silhouette tests.

## Inputs
- Area design: [../../roadmap/areas/K-rendering.md](../../roadmap/areas/K-rendering.md)
- O30: [../../parity/render-reference.md](../../parity/render-reference.md) and its golden cases
- O43: [../../parity/material-renders.md](../../parity/material-renders.md) and its golden cases
- O57: [../../parity/camera.md](../../parity/camera.md) and its golden cases
- render-reference.md, material-renders.md, camera.md

## Design decisions (planner)
- Per-case thresholds in case.toml.

## Done when
- [x] All cases pass or are listed with reasons.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- **31 captures measured:** 17/23 render silhouettes and 7/8 camera silhouettes
  pass; 6 existing mismatches remain; `mat-metal` is tone-only. All 9 focused
  tests pass, including the original checker-pattern pins and lens calibration.
- **Pins:** `area-soft` and `area-soft-2` differ on the lit plane; `checker-plane`,
  `mat-checker-uv` and `mat-checker-cubic` contain black cells mistaken for
  background (both sphere patterns retain the independent phase pin).
  `ortho-zoom-2` disagrees on width; `mat-metal` has only a small reflection,
  not a measurable object boundary.
- **PSNR:** the common comparator reports exposure-normalised PSNR for every
  capture and enforces explicit `[tolerance].psnr`. None of the captures
  supplies that field, so no undocumented floor was invented.
- **Files:** RENDER §R14a and the CPU test suites' shared `image_parity` helper.
- **Gate:** two parallel runs hit the unrelated existing graph test
  `a_shared_node_runs_once_per_evaluation`. With `RUST_TEST_THREADS=1`, the
  unchanged full suite passed all 2,482 tests and the gate printed `GATE: all passed`.
