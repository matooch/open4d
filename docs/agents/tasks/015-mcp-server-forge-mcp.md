# 015 · MCP server `forge-mcp`

Status: done · Area: A · Depends on: 014 · Observations: – · Touches: mcp (new)

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
- [x] Spec section in `docs/specs/RPC.md` §MCP.
- [x] An integration test that drives `forge-mcp --headless` over stdio: initialize, tools/list (contains `object_create`), tools/call object_create, then scene_get.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Acceptance S2 (019).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added: `crates/mcp/tests/stdio.rs` (8, spawning the real `forge-mcp` binary): headless initialize, `tools/list` (contains `object_create`, the five query tools, unique valid names, object input schemas), `object_create` then `scene_get`/`node_get`; `eval_mesh_summary` (8 points, 6 polys, centred bbox, no mesh data), `frame_set`, `export_obj` writing a file; refused commands and missing arguments as `isError` results, unknown tool/bad arguments/unknown method/malformed line/non-object/non-string method as protocol errors, notifications unanswered, server still serving; protocol version negotiation; `--headless <scene>` starts from a file (missing file exits 1, stdout stays clean); bad arguments exit 2 and `--help`; editor mode against a `forge_rpc::Server` on port 0 with a token (port from `--port` and from `FORGE_RPC_PORT`); wrong token and missing editor as `isError` (token never echoed). Four unit tests in `crates/mcp/src/lib.rs` (name mapping, schema wrapping, bbox summary, every registry command becomes a unique tool). All servers bind port 0. No parity data involved (N/A).
- Rules: spec `docs/specs/RPC.md` section 8. stdio, one JSON-RPC message per line; `initialize` (version negotiation over 2024-11-05, 2025-03-26, 2025-06-18), `notifications/*` ignored, `ping`, `tools/list`, `tools/call`. One tool per registry command (id with `.` replaced by `_`, schema = `args_schema`), generated from `command.list`, plus `scene_get`, `node_get`, `eval_mesh_summary`, `export_obj` (alias of `file.export_obj`), `frame_set` (alias of `timeline.set_frame`). Editor mode uses `forge_rpc::Client` (port/token as RPC.md section 2, lazy connect and reconnect); headless mode runs `forge_rpc::dispatch` on an in-process `App`, so both answer identically. Nothing parity-pending.
- Deviations from the design decisions:
  - Hand-written protocol layer instead of an SDK: the Rust MCP SDK (`rmcp`) is Apache-2.0 and maintained, but brings tokio, a proc-macro layer and schema generation for about 300 lines of protocol, and the schemas already exist in the command registry. The brief itself suggested hand-written.
  - Port and token are not passed as environment-only: `--port` was added next to `FORGE_RPC_PORT`; there is deliberately no `--token` argument (visible in `ps`).
  - `export_obj` and `frame_set` duplicate `file_export_obj` and `timeline_set_frame` (both appear in the list), as the brief names them explicitly.
  - Request/response batches and server-initiated messages (progress, logging, resources, prompts) are not implemented; the server declares only the `tools` capability.
  - A README.md was created at the repository root (none existed) for the Claude Code / Claude Desktop snippet.
- Files changed: crates/mcp/** (new: Cargo.toml, src/lib.rs, src/main.rs, tests/stdio.rs), README.md (new), Cargo.lock, docs/specs/RPC.md (section 8), this brief, docs/roadmap/BACKLOG.md.
- Open questions: whether tool results should also carry `structuredContent` (newer revisions); `tools/list` is cached for the process lifetime, so a plugin loaded later in the editor needs a restart (a `notifications/tools/list_changed` would fix that with 018 notifications).
- Gate: `GATE: 479 tests passed, 0 failed` / `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`
