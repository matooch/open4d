# 018 · RPC notifications, viewport screenshot, progress for long jobs

Status: done · Area: A · Depends on: 014 · Observations: – · Touches: rpc, ui

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
- [x] RPC.md updated.
- [x] Tests: subscribe, change the frame, receive the event; a screenshot of a cube is a valid PNG of the requested size, with lavapipe in CI.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: done. No parity data involved (own design). Tests added: `crates/rpc/tests/notifications.rs` (9: subscribe/frame, scene/selection/undo, unsubscribe and bad kinds, commands.changed, notifications between responses, screenshot errors and sizes, export job, cancel, job failures), `hub.rs` and `base64.rs` unit tests, `crates/ui/tests/rpc.rs` (+3: frame event through `EditorState`, cube screenshot PNG of the requested size, size checks), `crates/mcp/tests/stdio.rs` (+2: `tools/list_changed`, headless screenshot error), Python (+8: notifications, callback, screenshot error, jobs, wire test).
- Rules (spec RPC.md section 10): notifications are request-shaped lines without `id`: `scene.changed {revision}`, `frame.changed {frame}`, `selection.changed {nodes}`, plus `commands.changed {count}`; per-connection `events.subscribe/unsubscribe {kinds}` (kinds `scene|frame|selection|commands`, absent = all), answered by the connection thread. Detection compares revision/frame/selection/registry once or twice per `Inbox::drain` and per `run_until` iteration; events are coalesced. `viewport.screenshot {width,height}` -> `{width,height,format:"png",data:base64}`; `Host::screenshot` default errors (-32000, kind `viewport`); `EditorState` renders offscreen with its own lazily created headless `Gpu` through `render_to_rgba`. Jobs: `job.start {kind,args}` -> `{job_id}`, `job.progress {id,fraction,message}` and `job.finished {id,state,result|error}` to the starting connection, `job.cancel {id}`, `job.list`; jobs run one slice per drain/iteration on the document thread; kind `export.obj_sequence`.
- Deviations from the design decisions: added `commands.changed` and `job.finished` (needed for MCP list_changed and for delivering job results); `job.*` and `events.*` exist only over TCP (not through `dispatch`, so MCP headless has no jobs); jobs are cooperative slices on the UI thread, not worker threads, because the document is owned by that thread; the only job kind is a frame-sequence OBJ export (no renderer exists yet). Notification outbox is 1024 per connection; a reader that falls behind is disconnected. `App::export_obj_at(path, roots, frame)` added. MCP gets `viewport_screenshot` (image content) and `listChanged` in editor mode via a second RPC connection; Python client gets `subscribe`, `on_notification`, `next_notification`, `screenshot`, `job_*`.
- Files changed: docs/specs/RPC.md; crates/rpc (base64, hub, jobs, server, client, host, lib, tests); crates/app/src/lib.rs; crates/ui (screenshot.rs, state.rs, commands.rs, viewport_panel.rs, lib.rs, tests/rpc.rs); crates/mcp (lib, main, Cargo.toml, tests); python/forge_py (client, __init__, README, tests); README.md; this brief; BACKLOG.md.
- Open questions: the editor only checks for changes when it draws a frame, so a change made by another thread while the window is fully idle is reported at the next repaint; screenshots from a headless box need a GPU adapter (lavapipe works). More job kinds (render, bake) belong to the briefs that add those features.
- The last lines of the gate output:
  ```
  GATE: 548 tests passed, 0 failed
  Ran 37 tests ... OK (python)
  cleanroom: ok
  ok: 471 cases
  GATE: all passed
  ```
