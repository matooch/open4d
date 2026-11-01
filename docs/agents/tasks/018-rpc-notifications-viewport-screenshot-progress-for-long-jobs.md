# 018 · RPC notifications, viewport screenshot, progress for long jobs

Status: ready · Area: A · Depends on: 014 · Observations: – · Touches: rpc, ui

## Goal
Agents can watch the editor (scene, frame and selection changes), grab a viewport image, and follow long jobs.

## Inputs
- Area design: [../../roadmap/areas/A-automation.md](../../roadmap/areas/A-automation.md)
- brief 014

## Design decisions (planner)
- Notifications: `scene.changed {revision}`, `frame.changed {frame}`, `selection.changed {nodes}`, sent to subscribed connections (`events.subscribe {kinds}`).
- `viewport.screenshot {width, height}` returns a PNG as base64, rendered with the offscreen path the viewport tests already use.
- Long jobs return a `job_id` at once and report progress with `job.progress {id, fraction, message}`. A `job.cancel` request stops them.

## Done when
- [ ] RPC.md updated.
- [ ] Tests: subscribe, change the frame, receive the event; a screenshot of a cube is a valid PNG of the requested size, with lavapipe in CI.
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
