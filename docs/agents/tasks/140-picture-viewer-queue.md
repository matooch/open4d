# 140 · Picture viewer and render queue

Status: done (parity-pending: G8) · Area: K · Depends on: 139 · Observations: G8 · Touches: ui

## Goal
Picture viewer (history, A/B compare, channels) and a render queue.

## Inputs
- Area design: [../../roadmap/areas/K-rendering.md](../../roadmap/areas/K-rendering.md)
- GUI session G8 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G8 §3

## Design decisions (planner)
- Renders run off the UI thread with progress notifications (018).

## Done when
- [x] egui_kittest: render shows in history; A/B toggle.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- The Window menu opens Picture Viewer. Still jobs capture their input and run FIFO
  off-thread, including scheduled scene evaluation. CPU progress wakes the UI; waiting
  jobs can be removed and the active render can be cancelled. Successful renders go
  into bounded history, with A/B pins, AOV selection, exposure, and displayed PNG export.
- The egui test queues two CPU renders around a scene edit, verifies the images
  differ and include geometry, selects a Normal pass, and switches A/B while
  maintaining that pass without starting a new job.
- G8 §3 remains unobserved. The layout, channel display mappings and budgets are
  native choices. The queue handles stills; CLI sequence and multipass export are unchanged.
- Files: `crates/ui/src/picture_viewer.rs`, the Editor and Dock integration, UI acceptance
  test and RENDER §R17.
- Gate: `GATE: 2488 tests passed, 0 failed`; `GATE: all passed`.
