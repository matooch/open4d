# 012 · Headless application core `forge-app`

Status: done · Area: A · Depends on: 011 · Observations: – · Touches: app (new), ui

## Goal
A library that loads and saves scenes, runs commands, evaluates any frame, exports and (later) renders, without a window or a GPU. The editor and every automation front end build on it.

## Inputs
- Area design: [../../roadmap/areas/A-automation.md](../../roadmap/areas/A-automation.md)
- `crates/ui/src/state.rs` (`EditorState` today mixes UI and document state)
- brief 011 (`CommandContext`), brief 10 (`SceneSettings`, `scene_at`)

## Design decisions (planner)
- New crate `crates/app` (`forge-app`). `App { doc: Document, registry, commands, selection, mode, timeline, settings, eval_cache }`. It implements `CommandContext`.
- `EditorState` wraps `App` plus the UI-only state (dock, camera, expanded rows). Move everything that isn't UI into `App`.
- API: `App::new_default()` (all plugins installed, as `apps/studio/src/main.rs` does), `open(path)`, `save(path)`, `run(cmd_id, args) -> Result<Value>`, `eval_mesh(node, frame) -> PolyMesh` (world space), `export_obj(path, roots)`, `frame(n)`.
- Installing plugins moves from `apps/studio` to `forge_app::default_registry()`, so the CLI, tests and the editor share one place.

## Done when
- [x] Spec `docs/specs/APP.md`.
- [x] Tests: open a saved scene, run `object.create` and `param.set`, evaluate frame 15 of an animated cube, export OBJ, compare it with the harness parser.
- [x] The editor uses `App` and every UI test passes.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Rendering (K/135).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added: `crates/app/tests/app.rs` (7: default app has plugins and commands, open a saved scene, create and set_param are single undo steps, `eval_mesh` at frame 15 of an animated cube (and `frame` clamping), no-geometry error, `export_obj` at a frame compared with `forge_parity_harness::parse_obj`, headless component selection and key recording/jump). All forge-ui tests pass unchanged. No parity data involved (N/A).
- Rules: spec docs/specs/APP.md. `App` implements `CommandContext`; `EditorState { app, expanded, dock, camera, shading }` derefs to `App`, so `state.doc` etc. still work, and forwards its `CommandContext` to `App`, adding the view hooks.
- Deviations from the design decisions:
  - `App` has no separate `registry` or `settings` fields: the node registry and `SceneSettings` live in the document's scene (`doc.scene().registry()`, `settings()`); duplicating them would let them drift.
  - To make component selection, key recording and key navigation work headless (the open question of brief 011), `animate.rs` moved from forge-ui to `forge-app` (ui re-exports it as `forge_ui::animate`) and the UI-free parts of `components.rs` (`EditMode`, `SelectCmd`, `selection_of`, `write_selection`, `run_select`, grow/shrink, ...) moved to `forge_app::select`; the picking code stays in ui. `EditMode` <-> `ModeArg` and `SelectOp` -> `SelectCmd` conversions moved with them.
  - `frame(n)` sets the timeline frame and returns the evaluated scene; `eval_mesh` does not move the timeline.
  - `export_obj` exports the scene evaluated at the current frame; the `file.export_obj` command still exports the stored scene (unchanged behaviour).
  - `default_registry()` returns `Result` (plugin install can fail) and `apps/studio` no longer depends on generators/modifiers/core directly.
  - No `render` yet (out of scope, K/135).
- Files changed: crates/app/** (new), crates/ui/src/{state.rs,commands.rs,components.rs,lib.rs}, crates/ui/Cargo.toml, crates/ui/src/animate.rs (moved to crates/app/src/animate.rs), apps/studio/{src/main.rs,Cargo.toml}, Cargo.lock, docs/specs/APP.md (new), this brief, docs/roadmap/BACKLOG.md.
- Open questions: whether `file.export_obj` should export the evaluated frame too (needs a decision before the CLI, brief 013).
- Gate: `GATE: 444 tests passed, 0 failed` / `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`.
