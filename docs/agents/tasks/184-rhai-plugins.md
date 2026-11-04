# 184 · Rhai plugin packages

Status: done · Area: N · Depends on: 016 · Observations: – · Touches: plugin

## Goal
Folder plugins with a manifest that register Rhai node types and commands.

## Inputs
- Area design: [../../roadmap/areas/N-platform.md](../../roadmap/areas/N-platform.md)

## Design decisions (planner)
- Manifest TOML: name, version, entry script, permissions.

## Done when
- [x] Example plugin loads and adds a node type.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 12 tests in `crates/plugin/tests/plugin.rs`; no parity data involved (own design).
- Rules: manifest `plugin.toml` (name = namespace for all ids, version, entry, permissions
  [`commands`], node types with data-driven params, commands); node types are generators
  whose mesh comes from a Rhai build function in a pure sandbox (no scene, no commands);
  commands are Rhai functions run via `forge_script::run_function` (one undo step,
  rollback on failure); without `commands` permission the command budget is zero. Spec:
  docs/specs/PLUGINS.md.
- Deviations: the scene evaluator (`forge_generators::evaluate`) does not call
  `RhaiPlugin::evaluate` yet (forge-plugin depends on forge-script, which depends on
  forge-app, so the hook needs a design decision), and hosts (editor, CLI) do not call
  `install_all` yet. Plugin node types are created/edited like any node; meshes come from
  `RhaiPlugin::evaluate`. Also added `run_function`, `eval_function`, `script_functions`
  to forge-script.
- Files: crates/plugin/{Cargo.toml,src/lib.rs,src/manifest.rs,tests/plugin.rs},
  crates/script/src/lib.rs, docs/specs/{PLUGINS,SCRIPTING}.md,
  examples/plugins/ring-kit/*, this brief, BACKLOG.md.
- Open questions: where hosts load a `plugins/` folder (forge-platform data dir) and how
  evaluation should reach plugin generators (trait object hook in the scheduler?).
