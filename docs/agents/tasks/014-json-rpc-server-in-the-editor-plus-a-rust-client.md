# 014 · JSON-RPC server in the editor, plus a Rust client

Status: ready · Area: A · Depends on: 012 · Observations: – · Touches: rpc (new), ui, app

## Goal
A running editor can be driven over a localhost socket by the CLI, agents and scripts, with token authentication.

## Inputs
- Area design: [../../roadmap/areas/A-automation.md](../../roadmap/areas/A-automation.md)
- DECISIONS D4
- docs/agents/OBSERVER.md §1: the bridge lesson that an open port runs arbitrary code, so use a token and localhost only

## Design decisions (planner)
- New crate `crates/rpc` (`forge-rpc`): JSON-RPC 2.0, newline-delimited JSON over TCP on 127.0.0.1, default port 7323 (setting `rpc.port`). Uses tokio on its own thread.
- Auth: every request carries `"token"`, compared in constant time with the token from `FORGE_RPC_TOKEN` or the settings file. If no token is configured, the server doesn't start unless the editor runs with `--rpc-insecure-local`.
- Methods: `command.run {id, args}`, `command.list`, `scene.get` (the tree with params), `node.get {id}`, `eval.mesh {node, frame}`, `timeline.get`, `app.version`. Requests go to the UI thread over a channel and run between frames. The response comes after the command completes.
- Client: `forge_rpc::Client::connect(port, token)`, with typed helpers. `forge rpc` (013) uses it.
- Limits: 8 MB per message, 50 requests/s per connection, at most 4 connections.

## Done when
- [ ] Spec `docs/specs/RPC.md`.
- [ ] Tests: start the server on an ephemeral port with an in-process `App` (no window), connect, check the token is enforced, create a cube, set a param, read the scene, evaluate the mesh.
- [ ] Also test a bad token, an oversized message and a malformed request.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Notifications (018), MCP (015).

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
