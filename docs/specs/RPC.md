# JSON-RPC server and client (`crates/rpc`, package `forge-rpc`)

Source: our own design (docs/roadmap/areas/A-automation.md, DECISIONS D3 and D4;
docs/agents/OBSERVER.md §1 for the lesson that an open port runs arbitrary code). No
reference observation is involved. Nothing here is **parity-pending**.

A running editor can be driven over a localhost socket by the CLI, agents and scripts.
Every method is a thin layer over `forge_app::App` and the command registry
(COMMANDS.md), so the editor, `forge` and RPC share one implementation.

## 1. Transport and wire format

- TCP on `127.0.0.1` only (never another interface). Default port **7323**.
- Newline-delimited JSON: one JSON-RPC 2.0 object per line, UTF-8, `\n` terminated.
  A request must not contain a raw newline (JSON escapes them).
- Request: `{"jsonrpc":"2.0","id":1,"method":"scene.get","params":{},"token":"..."}`.
  `id` is a number or string; `params` is an object (or absent); `token` is a string
  (extension to JSON-RPC 2.0, see §2). A request without `id` is invalid (no
  notifications yet, brief 018).
- Success: `{"jsonrpc":"2.0","id":1,"result":<value>}`.
- Failure: `{"jsonrpc":"2.0","id":1,"error":{"code":-32602,"message":"..."}}`. `id` is
  `null` when the request could not be parsed. Responses come in request order, one per
  request, after the command completed.
- Batches (JSON arrays) are not supported: invalid request.

Error codes:

| code | meaning |
|---|---|
| -32700 | parse error (the line is not JSON) |
| -32600 | invalid request (not an object, no `id`, `method` not a string, wrong `jsonrpc`) |
| -32601 | unknown method |
| -32602 | invalid params (missing or wrongly typed field) |
| -32603 | internal error (handler panic, editor gone) |
| -32000 | the call ran and failed: a command refused, unknown node, node without geometry. `data` holds `{"kind": "command"\|"node"\|"build"\|...}` |
| -32001 | unauthorized (token missing or wrong); the connection is closed after the reply |
| -32002 | rate limited (request not executed) |
| -32003 | message too large; the connection is closed after the reply |
| -32004 | too many connections (sent instead of serving; then closed) |
| -32005 | the editor did not answer in time (30 s) |

## 2. Authentication

- Every request carries `"token"`. The server compares it to the configured token in
  constant time (no early exit; the loop runs over the longer of the two strings).
- The token comes from, in order: the environment variable `FORGE_RPC_TOKEN`; the settings
  file key `rpc.token`. An empty string counts as not set.
- With **no token configured the server does not start**, unless the editor was started
  with `--rpc-insecure-local`: then requests need no token. (A configured token is still
  required even with the flag.) The editor prints one line to stderr saying which case
  applies; it never prints the token.
- A request with a missing or wrong token gets -32001 and the connection is closed: no
  method runs and nothing about the scene is revealed.
- The token is never logged and never echoed in an error.

### Settings file
JSON at `$FORGE_SETTINGS` if set, else `$XDG_CONFIG_HOME/forge3d/settings.json`, else
`$HOME/.config/forge3d/settings.json`. Missing or unreadable is the same as empty.
```json
{"rpc": {"port": 7323, "token": "..."}}
```
Port: `FORGE_RPC_PORT`, else `rpc.port`, else 7323. (Port 0 asks the OS for a free port;
tests use it.) The file holds a secret: create it with mode 0600.

## 3. Limits

- 8 MiB per request line. A longer line is not read to the end: the server replies -32003
  and closes. (Responses have no limit.)
- 50 requests per second per connection, as a token bucket of capacity 50. Rejected
  requests (including bad ones) get -32002 and do not run.
- At most 4 connections at once; a fifth gets -32004 and is closed.
- A connection that sends nothing for 10 minutes is closed.

## 4. Methods

Node ids are integers, angles degrees, percent as shown (COMMANDS.md §4). Frames are
integers.

| method | params | result |
|---|---|---|
| `app.version` | - | `{"name": "forge3d", "version": "0.1.0", "protocol": 1}` |
| `command.list` | - | `{"commands": [{"id","label","undoable","args_schema"}]}` (same as `forge commands`) |
| `command.run` | `{"id": str, "args": object}` | `{"command": id, "result": <the command's result>}`; one undo step if the command edits |
| `scene.get` | - | `{"fps","start","end","frame","nodes","selected":[ids],"roots":[node...]}` with the node shape of CLI.md `info` (params in user units, `tags`, `animated`, `children`) |
| `node.get` | `{"id": n}` | the node object (same shape), -32000 if unknown |
| `eval.mesh` | `{"node": n, "frame": f}` (`frame` optional: the current frame) | `{"node","frame","points":[[x,y,z]],"polys":[[i...]]}` in world space, animation and deformers applied (`App::eval_mesh`); the timeline does not move |
| `timeline.get` | - | `{"frame","fps","start","end","playing","autokey"}` |

`command.run` runs the command registry's handler with the editor's own context, so view
commands (`view.shading`, ...) act on the open window. `file.*` commands take explicit
paths (no dialogs).

## 5. Threading in the editor

- `Server::start` binds the socket and spawns an accept thread and one thread per
  connection (at most 4). Connection threads only read lines, check limits and token,
  and parse JSON.
- A valid request becomes a `Job {method, params, reply}` on an `mpsc` channel. The
  connection thread then waits for the reply (30 s).
- The UI thread owns the document. Each frame, before drawing, the editor calls
  `Inbox::drain(&mut host)`, which runs at most 16 queued jobs through the `Host` trait
  (implemented by `App` and by the editor's `EditorState`) and sends each reply. Jobs run
  between frames, never during a draw, so a command is one undo step like a menu action.
- The server holds an optional wake callback (`Inbox::set_wake`); the editor sets it to
  `ctx.request_repaint()` so an idle window processes a job at once.
- Headless hosts (tests, a future `forge serve`) call `Inbox::run_until(&mut host, &stop)`.
- `Server::shutdown` (and `Drop`) stops the accept loop and closes connections.
- Deviation from the planner's note: plain `std::thread` and blocking sockets instead of
  tokio. With at most four connections nothing needs an async runtime, and it keeps the
  dependency tree and build time small. The wire protocol is unaffected.

## 6. Client

`forge_rpc::Client::connect(port, token: Option<&str>)` opens a connection to
`127.0.0.1:port`. `call(method, params) -> Result<Value, ClientError>` sends one request
and waits for its response (checks the id). `ClientError` is `Io`, `Protocol` (bad
response) or `Rpc {code, message}`. Typed helpers: `version`, `command_list`,
`command_run`, `scene_get`, `node_get`, `eval_mesh` (returns `MeshData {points, polys}`),
`timeline_get`.

`forge rpc <method> [--params JSON] [--port N] [--token T]` (CLI.md) uses it. Port and
token default as in §2 (`--token` is visible to other users via `ps`; prefer the
environment variable). A refused connection is exit 1 with `cannot connect to the editor
on port N`; an RPC error is exit 1 with its message. JSON:
`{"ok": true, "method": m, "result": <value>}`.

## 7. Tests (`crates/rpc/tests/rpc.rs`, `apps/forge/tests/cli.rs`, `crates/ui/tests`)

All servers bind port 0 and read the port back.
- Start a server with an in-process `App` (no window) on a pump thread; connect; a
  missing and a wrong token get -32001 and the connection closes; the right token works.
- Create a cube with `command.run object.create`, set a param with `param.set`, read it
  back with `scene.get` and `node.get`, `eval.mesh` returns 8 points and 6 polys and
  follows the param; `timeline.get`; `command.list` equals the registry; `app.version`.
- Malformed JSON (-32700), a non-object and a request without id (-32600), unknown method
  (-32601), bad params (-32602), a failing command (-32000); the connection stays usable
  after each.
- An oversized line (> 8 MiB) gets -32003 and the connection closes; the server still
  accepts new connections.
- The fifth connection is refused (-32004); rate limiting returns -32002 under a burst.
- No token configured: `Server::start` refuses unless insecure-local was asked.
- Constant-time comparison unit tests; settings-file parsing.
- `forge rpc` against a server started in the test (port 0, token in the environment).
- The editor's `EditorState` implements `Host` and answers `command.run` including
  `view.shading` (`crates/ui/tests/rpc.rs`).

## 8. MCP server (`crates/mcp`, package `forge-mcp`, binary `forge-mcp`)

Source: the public Model Context Protocol specification (stdio transport, `initialize`,
`tools/list`, `tools/call`, revisions 2024-11-05, 2025-03-26, 2025-06-18); our own design
otherwise (A-automation.md, DECISIONS D3). Nothing here is **parity-pending**.

AI agents use the editor through MCP. The server is a thin mapping from tools onto the RPC
methods of §4, so it adds no behaviour of its own.

### 8.1 Transport and protocol
- stdio: one JSON-RPC 2.0 message per line on stdin, replies one per line on stdout.
  stdout carries only protocol messages; diagnostics go to stderr. EOF on stdin ends the
  process with exit code 0.
- Hand-written (no SDK): the subset needed is `initialize`, `notifications/initialized`,
  `ping`, `tools/list` and `tools/call`; no async runtime, only `serde_json`. The MCP
  Rust SDK would pull in tokio and a macro layer for about 300 lines of protocol.
- `initialize`: reply `{"protocolVersion", "capabilities": {"tools": {}}, "serverInfo":
  {"name": "forge-mcp", "version"}, "instructions"}`. `protocolVersion` echoes the client's
  if it is one of the three revisions above, else the newest (2025-06-18).
- Messages without `id` are notifications and are never answered. `ping` returns `{}`.
  Unknown method: -32601. Malformed JSON: -32700 with `id` null. Not an object, or
  `method` not a string: -32600. Batches are not supported (-32600).
- `tools/call` for an unknown tool or bad `arguments` (not an object): -32602. A tool that
  ran and failed (command refused, unknown node, editor not reachable) is a normal result
  with `"isError": true` and the message as text, so the agent can read it.
- A successful call returns `{"content": [{"type": "text", "text": <JSON text>}],
  "isError": false}`.

### 8.2 Tools
Generated at `tools/list` from `command.list`, so a command added to the registry appears
without code changes.
- One tool per command: `name` = id with `.` replaced by `_` (`object.create` is
  `object_create`; ids contain only `[a-z0-9_.]`), `description` = label, whether it is
  undoable, and the original id, `inputSchema` = the command's `args_schema` (a schema
  without `"type": "object"` is wrapped). Call = `command.run`; the text is the command's
  result.
- Query tools: `scene_get` (= `scene.get`), `node_get {id}` (= `node.get`),
  `eval_mesh_summary {node, frame?}` (= `eval.mesh`, reduced to `{node, frame, points,
  polys, bbox: {min, max} | null}`: counts and bounding box, never the mesh),
  `export_obj {path, selected?}` (= command `file.export_obj`), `frame_set {frame}`
  (= command `timeline.set_frame`). A command whose mapped name equals one of these five is
  not listed.
- Names are unique; they match `[a-zA-Z0-9_]{1,64}`.

### 8.3 Connection
- Default: editor mode. The server connects to `127.0.0.1` with `forge_rpc::Client`; port
  and token resolve as in §2 (`FORGE_RPC_PORT`/`FORGE_RPC_TOKEN`, then the settings file);
  `--port N` overrides the port. The token is never taken from an argument (visible in
  `ps`) and never printed. The connection is made lazily on the first call and re-made after
  an I/O failure, so the MCP client can start before the editor; an unreachable editor is
  an `isError` tool result saying so.
- `--headless [scene]`: an in-process `forge_app::App` (opened from `scene` if given)
  answers through the same `dispatch` as the editor (`Host for App`); no port, no token.
  The document lives as long as the process; use `file_save_as` to keep it.
- Exactly one of the two modes per process; `--help` prints usage; a bad argument is exit 2.

### 8.4 Configuration
README.md (repository root) shows `claude mcp add` and `.mcp.json` for Claude Code and the
Claude Desktop config.

### 8.5 Tests (`crates/mcp/tests/stdio.rs` and unit tests)
- Spawn `forge-mcp --headless` and talk over its pipes: `initialize`, the initialized
  notification (no reply), `tools/list` contains `object_create`, `scene_get` and the
  other query tools with valid names; `tools/call object_create {"type": "forge.cube"}` then
  `scene_get` shows the node; `eval_mesh_summary` of a cube gives 8 points, 6 polys and its
  bbox; `frame_set` moves the frame; `export_obj` writes a file; a failing command is
  `isError` and the server keeps serving; unknown tool and unknown method; malformed line;
  EOF exits 0.
- A `--headless <scene file>` run starts with that scene.
- Editor mode against a `forge_rpc::Server` on port 0 with a token (pump thread with an
  `App`): tools work through the RPC client; a wrong token and a missing editor are
  `isError`.
- Unit tests: name mapping and collisions, schema wrapping, bbox.
