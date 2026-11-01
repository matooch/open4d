# 014 · JSON-RPC server in the editor, plus a Rust client

Status: done · Area: A · Depends on: 012 · Observations: – · Touches: rpc (new), ui, app

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
- [x] Spec `docs/specs/RPC.md`.
- [x] Tests: start the server on an ephemeral port with an in-process `App` (no window), connect, check the token is enforced, create a cube, set a param, read the scene, evaluate the mesh.
- [x] Also test a bad token, an oversized message and a malformed request.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Notifications (018), MCP (015).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added: `crates/rpc/tests/rpc.rs` (13: create a cube, set a param, read `scene.get`/`node.get`, `eval.mesh` follows the param, undo; `command.list` equals the registry; token enforced (missing, wrong, none; bad token runs nothing and is not echoed); no token means no server unless insecure-local; loopback only; malformed JSON, non-object, missing id, wrong `jsonrpc`, unknown method, bad params, failing command, with the connection staying usable; oversized message refused with the server still serving; a message just under 8 MiB accepted; fifth connection refused and a slot reusable; burst rate limit; dead editor gives an internal error; constant-time compare; settings resolution), `crates/ui/tests/rpc.rs` (1: `EditorState` as host, including `view.shading`), `apps/forge/tests/cli.rs` (+2: `forge rpc` against an in-process server, no editor listening). All servers bind port 0. No parity data involved (N/A).
- Rules: spec `docs/specs/RPC.md`. Wire: newline-delimited JSON-RPC 2.0 on 127.0.0.1 (default 7323), `"token"` member in each request. Methods: `app.version`, `command.list`, `command.run {id,args}`, `scene.get`, `node.get {id}`, `eval.mesh {node,frame?}`, `timeline.get`. Error codes -32700..-32601, -32602, -32603 plus -32000 (call failed, `data.kind`), -32001 unauthorized (connection closed), -32002 rate limited, -32003 too large (connection closed), -32004 too many connections, -32005 editor timeout. Limits: 8 MiB per line, 50 req/s per connection (token bucket), 4 connections. Token: `FORGE_RPC_TOKEN`, else settings file `rpc.token` (`$FORGE_SETTINGS`, else `$XDG_CONFIG_HOME/forge3d/settings.json`, else `~/.config/forge3d/settings.json`); no token means no server unless `--rpc-insecure-local`; compared in constant time. Port: `FORGE_RPC_PORT`, else `rpc.port`, else 7323. Threading: accept thread plus one thread per connection do I/O and validation; each valid request is a job on an `mpsc` channel; the editor drains up to 16 jobs per frame at the start of `Editor::show` through the `Host` trait (`App` and `EditorState` implement it, so `command.run` uses the editor's own context), and a wake callback requests a repaint. `forge rpc <method> [--params] [--port] [--token]` uses `forge_rpc::Client`.
- Deviations from the design decisions:
  - Plain `std::thread` and blocking sockets instead of tokio: at most four connections, so no async runtime is needed, and it avoids a large dependency and build time. The wire protocol does not depend on it.
  - `Client::connect(port, token)` takes `Option<&str>`.
  - Setting `rpc.port` and the token have no settings UI yet; there was no settings infrastructure, so a small JSON settings file plus environment variables were added (RPC.md §2).
  - `node_json`/`display_value` moved from the CLI to `forge_app::query` so `forge info` and RPC share them.
  - The `forge rpc` stub test in the CLI exit-code test was replaced by real rpc tests.
  - Not tested automatically: the real window serving requests (no display in CI); covered through `EditorState` plus `Inbox::drain`. `--rpc-insecure-local` is parsed by `forge-studio` only (not by `--headless`).
- Files changed: crates/rpc/** (new), crates/app/src/{lib.rs,query.rs}, crates/ui/{Cargo.toml,src/app.rs,src/commands.rs,src/lib.rs,tests/rpc.rs}, apps/forge/{Cargo.toml,src/lib.rs,tests/cli.rs}, apps/studio/{Cargo.toml,src/main.rs}, Cargo.lock, docs/specs/{RPC.md (new),CLI.md,APP.md}, this brief, docs/roadmap/BACKLOG.md.
- Open questions: response size has no limit (a huge `eval.mesh` is sent whole); notifications and screenshots come with 018; whether `rpc.allow_scripts` is needed once `script.run` exists (016).
- Gate: `GATE: 467 tests passed, 0 failed` / `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`
