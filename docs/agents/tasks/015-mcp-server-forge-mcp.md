# 015 · MCP server `forge-mcp`

Status: ready · Area: A · Depends on: 014 · Observations: – · Touches: mcp (new)

## Goal
AI agents use Forge3D through the Model Context Protocol: each command becomes an MCP tool, plus scene query tools.

## Inputs
- Area design: [../../roadmap/areas/A-automation.md](../../roadmap/areas/A-automation.md)
- brief 014
- the MCP specification (public): stdio transport, `tools/list`, `tools/call`

## Design decisions (planner)
- Binary crate `crates/mcp` (`forge-mcp`). Stdio transport, a minimal hand-written JSON-RPC MCP server (no heavy SDK needed).
- Mode 1 (default): connects to a running editor over RPC (port and token from env).
- Mode 2 `--headless [scene]`: runs an in-process `App`, so agents can work without a GUI.
- Tools: one per registry command (name = id with dots replaced by underscores; input schema = the command's schema), plus `scene_get`, `node_get`, `eval_mesh_summary` (counts and bbox, not the full mesh), `export_obj`, `frame_set`.
- A `README` snippet that shows how to register it with Claude Code (`.mcp.json`) and Claude Desktop.

## Done when
- [ ] Spec section in `docs/specs/RPC.md` §MCP.
- [ ] An integration test that drives `forge-mcp --headless` over stdio: initialize, tools/list (contains `object_create`), tools/call object_create, then scene_get.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Acceptance S2 (019).

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
