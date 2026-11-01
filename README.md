# Forge3D (open4d)

A native Linux, Rust reimplementation of a classic DCC application. See `CLAUDE.md` and
`docs/` for the plan, specs and rules.

## Use Forge3D from an AI agent (MCP)

`forge-mcp` is a Model Context Protocol server (stdio). Every editor command becomes a
tool (`object_create`, `param_set`, ...), plus `scene_get`, `node_get`,
`eval_mesh_summary`, `export_obj` and `frame_set`. Spec: `docs/specs/RPC.md` section 8.

Build it: `cargo build --release -p forge-mcp` (binary `target/release/forge-mcp`).

Two modes:

- **Editor** (default): forwards to the running editor over RPC on `127.0.0.1`. Start the
  editor with a token, and give the same token to the server:
  `FORGE_RPC_TOKEN=change-me forge-studio`. Port: `FORGE_RPC_PORT` or `--port` (default
  7323). The token is read from `FORGE_RPC_TOKEN` or the settings file, never from an
  argument.
- **Headless**: `forge-mcp --headless [scene-file]` runs an in-process app, no editor and
  no token needed. Save your work with the `file_save_as` tool.

### Claude Code

```sh
# editor mode
claude mcp add forge3d --env FORGE_RPC_TOKEN=change-me -- /path/to/forge-mcp
# headless mode
claude mcp add forge3d-headless -- /path/to/forge-mcp --headless
```

or a project-level `.mcp.json`:

```json
{
  "mcpServers": {
    "forge3d": {
      "command": "/path/to/forge-mcp",
      "args": [],
      "env": { "FORGE_RPC_TOKEN": "change-me" }
    },
    "forge3d-headless": {
      "command": "/path/to/forge-mcp",
      "args": ["--headless", "scene.forge"]
    }
  }
}
```

### Claude Desktop

Add the same `mcpServers` entries to `claude_desktop_config.json` and restart the app.
