# 145 · Render parity tests against O30/O43/O57

Status: waiting: O30, O43, O57 · Area: K · Depends on: 137, 138 · Observations: O30, O43, O57 · Touches: tests

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
- [ ] All cases pass or are listed with reasons.
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
