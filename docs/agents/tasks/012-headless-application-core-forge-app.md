# 012 · Headless application core `forge-app`

Status: ready · Area: A · Depends on: 011 · Observations: – · Touches: app (new), ui

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
- [ ] Spec `docs/specs/APP.md`.
- [ ] Tests: open a saved scene, run `object.create` and `param.set`, evaluate frame 15 of an animated cube, export OBJ, compare it with the harness parser.
- [ ] The editor uses `App` and every UI test passes.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Rendering (K/135).

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
