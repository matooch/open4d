# 108 · Node expression engine forge-nodes

Status: done · Area: H · Depends on: 016 · Observations: – · Touches: nodes (new)

## Goal
A typed node graph engine for expressions (and later node materials).

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)

## Design decisions (planner)
- Typed ports, topological evaluation, cycle check, node types registered through the plugin registry; Rhai node for custom code (016).

## Done when
- [ ] Unit tests: math graph evaluates, cycle rejected, graph serializes.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
UI (109).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: new crate `crates/nodes` (`forge-nodes`), spec `docs/specs/NODES.md`. 50 new tests (`ports.rs`, `graph.rs`, `nodes.rs`, `rhai.rs`, `expression.rs`), plus a v7 -> v8 migration test and v8 fixtures in core. No parity cases (own design).
- Rules: typed ports (float, int, bool, vec3, colour, matrix, string, node, any) with a fixed implicit table and explicit `convert.*` nodes; graph with stable never-reused ids, one link per input, type and cycle checks on connect, Kahn topological order (ties by id), each node runs once per evaluation, only sinks and their dependencies run. 38 built-in node types incl. noise (forge-noise) and a sandboxed Rhai node (limits equal `forge_script::ScriptOptions::default()`, asserted by a test). Expression tag `forge.expression` stores the graph as a hidden JSON string param; `apply_expressions` runs tags in document order after animation and before generators (CORE.md 5a). Commands `graph.add_node|remove|connect|disconnect|set_value|get|node_types|evaluate`, each edit one undo step.
- Deviations: (1) forge-nodes cannot depend on forge-script (cycle through forge-app), so it pins the same rhai and repeats the limits, with a drift test (forge-script as dev-dependency). (2) Commands use the built-in node registry only. (3) File format bumped to 8 although the schema is unchanged (graph is a string param), so older builds reject rather than silently drop expressions; existing tests changed only for version numbers. (4) Tag priorities are document order until brief 111.
- Files: crates/nodes/**, crates/app/{Cargo.toml,src/lib.rs,src/animate.rs}, crates/core/src/file.rs, crates/core/tests/{file,tracks,time,native_format}.rs, fixtures scene-v8*.forge, docs/specs/{NODES,CORE,COMMANDS,SCRIPTING}.md, docs/manual/reference/*, BACKLOG.md, Cargo.lock.
- Open questions: tag priority model (111); whether the UI (109) needs custom node plugins in commands.
- Gate: `GATE: 1244 tests passed, 0 failed` ... `cleanroom: ok` ... `ok: 471 cases` ... `GATE: all passed`.
