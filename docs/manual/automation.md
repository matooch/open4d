# Automate Forge3D

Everything you can do in the editor is a **command** (an id such as `object.create` plus a JSON
arguments object). Four front ends run the same commands, so a script, an agent and a person
all get the same result and the same undo history:

| Front end | Use it for | Needs the editor running? |
|---|---|---|
| `forge` CLI | batch work on scene files, CI, quick checks | no |
| JSON-RPC (+ `forge-py`) | tools that drive a live editor | yes |
| `forge-mcp` | AI agents (Model Context Protocol) | yes, or `--headless` |
| Rhai scripts (`forge run`) | building a scene from code, repeatable recipes | no |

The generated references list every command, option, method and function. They are produced by
the code and checked by the tests, so they cannot drift:

- [Commands](reference/commands.md): ids, labels, argument tables
- [CLI](reference/cli.md), [RPC](reference/rpc.md), [MCP](reference/mcp.md), [Rhai](reference/scripting.md)

Regenerate them after changing a command, option or binding:

```sh
cargo run -p forge-cli -- docs --out docs/manual/reference
```

`cargo test -p forge-cli --test docs` (part of the gate) fails while the committed files are stale.

Units everywhere: distances in scene units, **angles in degrees**, percentages as shown,
node ids are integers, frames are integers.

## Setup

```sh
cargo build --release -p forge-cli -p forge-mcp -p forge-studio
# binaries: target/release/forge, forge-mcp, forge-studio
```

`forge-studio --headless <forge args>` runs the CLI too, so one installed binary is enough.

### The token

The editor's RPC server (used by `forge rpc`, `forge-py` and `forge-mcp` in editor mode) listens
on `127.0.0.1:7323` and **refuses every request without the token**. It does not start at all
without one, because an open port that runs commands is a remote-control hole.

```sh
export FORGE_RPC_TOKEN="$(head -c 16 /dev/urandom | od -An -tx1 | tr -d ' \n')"
forge-studio            # prints: rpc: listening on 127.0.0.1:7323 (token required)
```

The token and port can also live in the settings file (`~/.config/forge3d/settings.json` on Linux,
`~/Library/Application Support/forge3d/settings.json` on macOS, `%APPDATA%\forge3d\settings.json`
on Windows, or `$FORGE_SETTINGS`; keep it readable by you only):

```json
{"rpc": {"port": 7323, "token": "change-me"}}
```

Order: `FORGE_RPC_TOKEN` and `FORGE_RPC_PORT` first, then the file. Clients read the same two
places, so exporting the variable once in your shell is enough. Never put the token on a command
line you share (`forge rpc --token` shows up in `ps`). For throw-away local experiments,
`forge-studio --rpc-insecure-local` starts the server without a token.

## 1. The `forge` CLI

No editor, no window, no GPU. Add `--json` for one JSON object on stdout (`"ok": true` or an
error with an exit code); exit codes: 0 ok, 1 user error, 2 scene cannot be loaded, 3 internal.

```sh
forge new scene.forge
forge cmd scene.forge object.create --args '{"type":"forge.cube"}' --save
forge cmd scene.forge param.set --args '{"node":1,"param":"size_x","value":300}' --save
forge info scene.forge                   # the node tree
forge eval scene.forge --frame 15        # counts and bounding boxes at frame 15
forge export scene.forge --format obj --out cube.obj --frame 15
forge --json commands                    # every command with its argument schema
```

`cmd` is a dry run unless you pass `--save`. Selection is not stored in scene files, so give
`nodes` explicitly to commands that default to the selection.

## 2. JSON-RPC and `forge-py`

One JSON-RPC 2.0 object per line over TCP; every request carries `"token"`.

```sh
forge rpc scene.get                                  # uses FORGE_RPC_TOKEN / FORGE_RPC_PORT
forge rpc command.run --params '{"id":"object.create","args":{"type":"forge.sphere"}}'
forge rpc viewport.screenshot --params '{"width":640,"height":360}'
```

`events.subscribe` turns on `scene.changed`, `frame.changed`, `selection.changed` and
`commands.changed` notifications; `job.start` runs long tasks (an OBJ frame sequence) with
`job.progress` and `job.finished`. See the [RPC reference](reference/rpc.md).

The Python client (`python/forge_py`, standard library only) turns every command into a method:

```python
from forge_py import Client

with Client() as forge:                      # port and token as above
    sphere = forge.object_create(type="forge.sphere")["node"]
    forge.timeline_settings(fps=30, start=0, end=30)
    forge.key_set(node=sphere, track="transform.pos.x", frame=0, value=0)
    forge.key_set(node=sphere, track="transform.pos.x", frame=30, value=300)
    forge.timeline_set_frame(frame=15)
    print(forge.eval_mesh(sphere).bbox())    # evaluated at the current frame
```

Without an editor, `cargo run -p forge-rpc --bin forge-rpc-serve` is the same server with no
window (prints `FORGE_RPC_PORT=<port>`).

## 3. MCP for agents

`forge-mcp` is a stdio MCP server. Every command is a tool (`object.create` becomes
`object_create`), plus `scene_get`, `node_get`, `eval_mesh_summary`, `export_obj`, `frame_set`
and `viewport_screenshot` ([list](reference/mcp.md)).

```sh
# against the running editor (the token comes from the environment, never an argument)
claude mcp add forge3d --env FORGE_RPC_TOKEN=change-me -- /path/to/forge-mcp
# a private in-process app, no editor needed; save with the file_save_as tool
claude mcp add forge3d-headless -- /path/to/forge-mcp --headless
```

A typical agent session: call `scene_get`, create nodes with `object_create`, change them with
`param_set` / `transform_set`, key them with `key_set`, read the result with
`eval_mesh_summary`, look at it with `viewport_screenshot`, undo mistakes with `edit_undo`.
Errors come back as tool results with `isError`, so the agent can read them and retry.

## 4. Rhai scripts

A script is registry commands plus read-only scene queries. `forge run` is a dry run unless
you save:

```rhai
// bend.rhai
run("timeline.settings", #{ fps: 30, start: 0, end: 60 });
let cube = create("forge.cube", "Cube");
let bend = create("forge.bend", "Bend");
run("object.reparent", #{ node: bend, parent: cube });
run("key.set", #{ node: bend, track: "strength", frame: 0,  value: 0.0 });
run("key.set", #{ node: bend, track: "strength", frame: 60, value: 90.0 });
print(`cube is node ${cube}`);
```

```sh
forge run bend.rhai --save bend.forge
forge eval bend.forge --frame 30 --export bend30.obj
```

A run is one undo step; a failed run changes nothing. Scripts have no file, network or clock
access, a seeded `rand(seed)` for reproducible randomness, and limits on operations, time and
commands ([reference](reference/scripting.md)). More examples: `examples/scripts/`. The same
engine runs in the editor's Script Manager panel.

## Acceptance scenarios

Two scenarios from the roadmap run in CI and double as worked examples:

- **S1** (`tests/acceptance/s1.rhai`, `apps/forge/tests/acceptance.rs`): a script builds a cube
  with an animated bend; `forge` evaluates frame 30 and exports it. The cloner with a random
  effector (briefs 065, 069), glTF export (164) and the PNG render (136, 139, 146) are marked
  pending in that file.
- **S2** (`crates/mcp/tests/stdio.rs`, `python/forge_py/tests/test_acceptance.py`): an agent
  creates a sphere, keys position X at frames 0 and 30, plays the timeline and reads back the
  evaluated position at frame 15, through MCP (editor and headless) and through `forge-py`.
