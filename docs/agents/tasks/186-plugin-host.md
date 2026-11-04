# 186 · Plugin host integration: evaluation hook and plugin folders

Status: ready · Area: N · Depends on: 184 · Observations: – · Touches: core, generators, plugin, app, cli, ui

## Goal
Plugin node types from brief 184 produce geometry in the live scene (viewport, scheduler, export, `forge` CLI), and hosts load plugins from a known folder without extra code.

## Inputs
- [../../specs/PLUGINS.md](../../specs/PLUGINS.md) §6 (open gaps left by 184)
- `crates/plugin` (`RhaiPlugin::evaluate`, `discover`, `install_all`, `bind`)

## Design decisions (planner)
- **Evaluation hook in `forge-core`**, so there is no dependency cycle: the `Registry` holds an optional evaluator per node type (`Arc<dyn NodeEvaluator + Send + Sync>`, a trait with `evaluate(&self, scene, id, ctx) -> Result<Option<PolyMesh>, _>`; put the trait wherever `PolyMesh` is already visible to core, or return the geometry type core already uses). `forge_generators::evaluate` and the scheduler consult it when a type has no built-in evaluator. forge-plugin registers its evaluator at install time.
- Evaluation stays sandboxed and deterministic (the 184 build-function sandbox and caps unchanged). A failing plugin evaluation is a node error, never a panic or a host crash.
- **Plugin folders**, loaded in this order, later folders skipping ids already taken: `<forge-platform data dir>/plugins/`, then every folder in `FORGE_PLUGIN_PATH` (OS path-list separator), then `--plugins <dir>` (repeatable) on the `forge` CLI. `--no-plugins` and `FORGE_NO_PLUGINS=1` disable all loading (tests and CI use this when isolation matters).
- The editor, the CLI, the RPC server and MCP all go through one `forge_app` (or forge-plugin) entry point; headless and windowed load the same set.
- A `plugin.list` command (read-only, no undo step) reports loaded plugins, their node types and commands, and the skipped ones with the reason.

## Done when
- [ ] A test scene with the example `ring-kit` node evaluates through `forge_generators::evaluate` and the scheduler, and exports to OBJ, with the same mesh as `RhaiPlugin::evaluate`.
- [ ] Folder order, the env var, `--plugins`, `--no-plugins` and clash skipping are tested (temp dirs; never the real data dir).
- [ ] `plugin.list` works over CLI and RPC; the generated command reference is regenerated.
- [ ] PLUGINS.md §6 is closed; the gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added; parity N/M.
- Deviations from the design decisions, and why.
- Files changed.
- Open questions.
- The last lines of the gate output.
