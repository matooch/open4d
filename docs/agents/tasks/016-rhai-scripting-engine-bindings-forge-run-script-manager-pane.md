# 016 · Rhai scripting: engine, bindings, `forge run`, Script Manager panel

Status: done · Area: A · Depends on: 012 · Observations: – · Touches: script (new), apps/forge, ui

## Goal
Users and agents can write scripts that build and edit scenes. Scripts run from the CLI and from a Script Manager panel in the editor.

## Inputs
- Area design: [../../roadmap/areas/A-automation.md](../../roadmap/areas/A-automation.md)
- DECISIONS D5
- brief 012 (`App`), brief 011 (commands)

## Design decisions (planner)
- New crate `crates/script` (`forge-script`) with `rhai` (sync, no std features we don't need).
- Bindings: `run(id, #{args})` → command; `scene()` queries (`nodes()`, `node(id).param(name)`, `children`, `find(name)`); maths types `vec3`, `deg(x)`; `frame(n)`; `print`; a seeded `rand(seed)`.
- Safety: no file system access except `export_*` commands with explicit paths; max operations limit; a timeout.
- `forge run script.rhai [--scene in.forge] [--save out.forge]`.
- Script Manager panel: a code editor (egui `TextEdit` with monospace; syntax colouring optional), Run, an output console, and a recent scripts list stored in the settings.
- Example scripts in `examples/scripts/` (cube grid, animated sphere). They also serve as tests.

## Done when
- [x] Spec `docs/specs/SCRIPTING.md` with the binding reference.
- [x] Tests run every example script headlessly and check the resulting scene.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Rhai plugin packages (N/184), script nodes (H/108).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: new crate `forge-script` (rhai pinned `=1.27.0`, MIT OR Apache-2.0; features `no_module`, `no_time`; transitive `smartstring` is MPL-2.0). 29 script tests, 4 CLI tests, 6 UI tests, 3 core undo tests, 1 app test (`key.set`). No parity cases (own design).
- API (docs/specs/SCRIPTING.md): `run(id[, #{args}])`, `commands()`, `create(type[, name])`, `set_param`, `set_transform`, `scene()` (`nodes/roots/find/find_all/node/selection/len`, `fps/start/end`), node handles (`id/name/type/parent()/children()/param()/params()`), `frame()`/`frame(n)`, `vec3` with operators, `deg`/`to_deg`, `rand(seed)` (SplitMix64: `float/range/int`), `print`/`debug`.
- Undo: one run is one `Run Script` step (`Document::group_undo_since`); a failed run is rolled back (`Document::rollback_to`), leaving no redo. `edit.undo/redo`, `file.new/open/save/save_as/import_obj` are refused in scripts; `file.export_*` allowed.
- Sandbox: no modules/imports, `eval` disabled, no clock; limits 5 000 000 operations, 10 s, 10 000 commands, call depth 48, expr depth 64/32, string 1 MiB, array/map 100 000. Limits are not catchable by `try/catch`.
- Deviations: (1) added the command `key.set {node, track, frame, value}` (COMMANDS.md): `key.record` keys the evaluated value so an animated example could not be authored with the existing commands. (2) Added `Document::undo_depth/group_undo_since/rollback_to` (CORE.md §4) and exported `forge_rpc::settings_path`. (3) The Script panel is closed by default (`Panel::DEFAULT`) so the existing default layout and its tests are unchanged. (4) `forge run` has no timeout flag; defaults only.
- Files changed: crates/script/**, examples/scripts/*.rhai, docs/specs/{SCRIPTING,CLI,COMMANDS,CORE,UI}.md, apps/forge, crates/{core,command,rpc,app (tests),ui}.
- Open questions: RPC/MCP `script.run` (needs the `rpc.allow_scripts` setting) is left for a later brief; script-supplied arguments for `forge run` are not defined.
- Gate: `GATE: 529 tests passed, 0 failed` / `GATE: all passed`.
