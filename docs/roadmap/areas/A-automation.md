# A · Automation: commands, CLI, RPC, MCP, scripting

**Why first:** everything else gets cheaper once agents can drive the app. Implementers
test features through the CLI, observers' results are replayed through it, and the owner
gets the "agents build things in the app for me" capability early.

## Scope (v1)

- [ ] **Command registry** (DECISIONS D3).
  - Every user action is a `Command`: a stable id (`object.create`, `param.set`,
    `select.grow`, `key.record`, `file.export.obj` …), a JSON-schema argument description,
    a typed Rust struct, undoable through `Document::transact`, and a result value.
  - Registered by plugins like node types.
  - Menus, shortcuts and the command palette list commands from the registry.
- [ ] **Headless application core** (`forge-app` crate):
  - open, save, evaluate a scene at a frame, run commands, export, render;
  - no window, no GPU required (the software path where possible).
- [ ] **`forge` CLI** (same binary as the editor; `forge-studio --headless` is an alias):
  - `forge new|info|validate|eval|export|render|run|cmd|rpc|serve`;
  - JSON output for agents (`--json`);
  - exit codes per error class.
- [ ] **JSON-RPC 2.0 server** inside the running editor:
  - localhost TCP, port 7323 by default, configurable;
  - a token from the environment or the settings file, required for every call (the
    lesson from the reference-app bridge: an open port runs arbitrary code);
  - methods map 1:1 onto commands, plus a few queries: `scene.get`, `node.get`,
    `eval.mesh`, `viewport.screenshot`, `timeline.get`;
  - notifications: `scene.changed`, `frame.changed`, `selection.changed`.
- [ ] **MCP server** `forge-mcp`: stdio MCP. Each command becomes an MCP tool with its
  JSON schema. It connects to a running editor over RPC, or starts a headless core.
- [ ] **Rhai scripting**:
  - an engine with bindings for the command layer, scene queries and maths types;
  - `forge run script.rhai`;
  - a Script Manager panel (edit, run, console);
  - script nodes later (H, node expressions).
- [ ] **Python client** `forge-py`: a pure-Python package that talks RPC (no embedding).
- [ ] **Docs**:
  - a generated command reference (from the registry);
  - CLI, RPC and MCP guides;
  - examples that are also the S1/S2 acceptance tests.

## Observation jobs
None: this is our own design. It is still level C: the command names follow the reference
app's menu names, so the user can find things (taken from the public manual and the
GUI session G1).

## Implementation design

- **Crates:**
  - `crates/command` (`forge-command`): the `Command` trait, `CommandRegistry`, argument
    schema (`schemars`), `CommandContext` (document, selection, timeline, editor state
    access via traits so headless works), and results.
  - `crates/app` (`forge-app`): `App` = document + registry + settings + evaluation
    cache. No egui.
  - `crates/rpc` (`forge-rpc`): server with `tokio`, line-delimited JSON over TCP, token
    auth, and a client.
  - `crates/mcp` (`forge-mcp`, binary): MCP over stdio, with tool list = commands.
  - `crates/script` (`forge-script`): the Rhai engine and bindings.
- **Migration:**
  - The existing `ui::commands::Command` enum becomes registry commands. The editor's
    `execute` forwards to the registry.
  - Gizmo drags stay interactive. On release they are recorded as a `transform.set`
    command, so they can be replayed.
- **Threading:** the editor runs RPC on a background runtime and posts requests to the UI
  thread through a channel. Requests run between frames, and the response is sent after
  the command completes. Long jobs (render, export) report progress notifications.
- **Security:**
  - the server binds to 127.0.0.1 only;
  - the token is required (the server refuses to start without one unless
    `--insecure-local` is passed);
  - per-connection rate limit;
  - no `script.run` over RPC unless the setting `rpc.allow_scripts = true`.
- **Determinism:** commands take explicit seeds where randomness exists. Scripts get a
  seeded RNG.

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 011 | Command registry and migration of the editor's commands | – | – |
| 012 | Headless application core `forge-app` | 011 | – |
| 013 | `forge` CLI | 012 | – |
| 014 | JSON-RPC server in the editor + Rust client | 012 | – |
| 015 | MCP server `forge-mcp` | 014 | – |
| 016 | Rhai scripting: engine, bindings, `forge run`, Script Manager panel | 012 | – |
| 017 | Python client `forge-py` | 014 | – |
| 018 | RPC notifications, viewport screenshot, progress for long jobs | 014 | – |
| 019 | Automation docs, generated command reference, S1/S2 acceptance tests | 013, 015, 016 | – |

## Done when
S1 and S2 pass headless in CI on Linux, and on macOS and Windows runners once N/173 is in.
Every menu item in the editor is a registry command.
