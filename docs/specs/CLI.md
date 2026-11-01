# `forge` command-line tool (`apps/forge`, package `forge-cli`)

Source: our own design (docs/roadmap/areas/A-automation.md, DECISIONS D3); no reference
observation is involved. Nothing here is **parity-pending**.

`forge` is the headless front end for people and agents. It is a thin layer over
`forge_app::App` (APP.md) and the command registry (COMMANDS.md); it needs no window or
GPU. The code lives in a library (`forge_cli::run`) so the editor binary can forward
`forge-studio --headless <forge args>` to the same function: one installed binary does both.

## 1. Conventions

- Syntax: `forge [--json] <subcommand> ...`. `--json` is global (may follow the
  subcommand). Scene files are the `.forge` files of CORE.md.
- With `--json`, the result is **one JSON object on stdout**; human messages (progress,
  warnings, errors) go to stderr. Without it, stdout holds human text.
- A JSON failure is `{"ok": false, "error": {"code": N, "message": "..."}}` on stdout, and
  the message on stderr. Every success object has `"ok": true`.
- Node ids are integers. Angles are degrees, percent as shown (COMMANDS.md §4).
- Exit codes: `0` ok; `1` user error (bad arguments, unknown command id or node, a command
  that refuses, unwritable output, a stub subcommand, a failed `rpc` call, a failed script); `2` the scene file cannot be read
  or loaded; `3` internal error (a bug or panic).
- Frames are integers, clamped to the scene's range (APP.md §2).

## 2. Subcommands

### `forge new <path> [--force]`
Writes an empty scene. Refuses to overwrite an existing file unless `--force`.
JSON: `{"ok": true, "path": "<path>"}`.

### `forge info <scene>`
The node tree. Text: one indented line per node (`id type "name"`), tags in brackets.
JSON:
```json
{"ok": true, "path": "...", "fps": 30, "start": 0, "end": 90, "nodes": 2,
 "roots": [{"id": 1, "type": "forge.cube", "name": "Cube",
            "params": {"size_x": 200.0, "transform.pos": [0.0, 0.0, 0.0]},
            "tags": [{"id": 3, "type": "forge.smoothing", "name": "...", "params": {}}],
            "animated": ["transform.pos.x"],
            "children": [ ... same shape ... ]}]}
```
`params` holds every non-mesh parameter in user units; a mesh parameter is
`{"points": n, "polys": m}`. `animated` lists track keys (omitted when empty).
`nodes` counts all nodes including tags.

### `forge validate <scene>`
Loads the scene and builds every node's geometry. Load warnings are printed and listed;
they do not fail. A node whose generator rejects its parameters is an error: exit 1.
A file that cannot be loaded is exit 2.
JSON: `{"ok": true|false, "warnings": [..], "errors": [{"node": 4, "message": ".."}]}`.

### `forge eval <scene> --frame N [--node ID] [--export out.obj]`
Evaluates the scene at frame `N` (default 0). Without `--export` it reports each node
that has geometry (all of them, or only `--node`; consumed inputs skipped):
`{"ok": true, "frame": N, "nodes": [{"id", "name", "points", "polys", "bbox": [[min xyz], [max xyz]]}]}`
with the bounding box of the world-space points. With `--export` it also writes the OBJ
exactly like `export` (below) and adds `"export": {"path", "objects"}`. An unknown node is
exit 1; a node without geometry is exit 1 (`node N has no geometry`).

### `forge export <scene> --format obj --out <path> [--selected ID,ID,...] [--frame N]`
Writes the scene **evaluated at frame `N`** (default 0) as OBJ (`App::export_obj`; the
same as `file.export_obj` at that frame). `--selected` limits it to those nodes and their
descendants. Only `obj` is accepted. Nothing to export is exit 1.
JSON: `{"ok": true, "path": "<out>", "objects": n, "frame": N}`.

### `forge cmd <scene> <command-id> [--args JSON] [--save]`
Opens the scene, runs one command with the JSON object `--args` (default `{}`), and
prints its result: `{"ok": true, "command": "<id>", "result": <value>}`. The file is
rewritten only with `--save` (so a plain run is a dry run). A failing command changes
and saves nothing (exit 1). Selection is not stored in scene files, so commands that take
an optional `nodes` list must be given it explicitly.

### `forge commands`
Lists the registry. Text: `id  label` per line. JSON:
`{"ok": true, "commands": [{"id", "label", "undoable", "args_schema": {..}}]}`.

### `forge rpc <method> [--params JSON] [--port N] [--token T]`
Calls a method of a running editor over JSON-RPC (RPC.md; methods in RPC.md §4).
`--params` is a JSON object (default `{}`). Port and token default to `FORGE_RPC_PORT` /
`FORGE_RPC_TOKEN`, then the settings file, then 7323 / none. JSON:
`{"ok": true, "method": "...", "result": <value>}`; text output is the result pretty
printed. No editor listening, a wrong token, an unknown method or a failing call is exit 1
with the message.

### `forge run <script.rhai> [--scene s] [--save out.forge]`
Runs a Rhai script (SCRIPTING.md) on the scene `--scene` (default: an empty scene) with the
default sandbox limits. `print` output goes to stdout, one line each (text mode). The
scene is written only with `--save <path>` (so a plain run is a dry run; the path may be
the input file). The whole run is one undo step, which has no effect on the saved file.
JSON: `{"ok": true, "script": "<path>", "output": [lines], "value": <last expression>,
"commands": n, "saved": "<path>"|null}`. Text mode prints the output lines, then nothing
else. A failed script (syntax error, runtime error, limit) saves nothing and is exit 1;
the message reads `<script>: line N: <message>` and the printed lines go to stderr. A script
file that cannot be read is exit 1; a scene that cannot be loaded is exit 2.

### `forge render`
Stub. It prints `render is not available yet (area K)` and exits 1.

## 3. Editor forwarding

`forge-studio --headless <args...>` runs `forge_cli::run` with the remaining arguments
and exits with its code. Without `--headless` the editor starts.

## 4. Tests (`apps/forge/tests/cli.rs`, `assert_cmd`)

- `new` -> `cmd object.create {"type":"forge.cube"} --save` -> `eval --frame 0 --export` ->
  the OBJ equals `tests/golden/cube/defaults/mesh.obj` under `compare_meshes`; `export`
  gives the same file.
- `info --json` lists the cube; `commands --json` lists every registry id; `validate` ok,
  a corrupt file exits 2.
- An animated cube exports at frame 15 moved; at frame 0 at the origin (evaluated export).
- Exit codes: unknown command id and bad args exit 1, missing scene exits 2, stubs exit 1;
  `cmd` without `--save` leaves the file unchanged; JSON errors carry the code.
- `file.export_obj` of an animated scene writes the evaluated frame (`crates/app/tests`).
- `forge rpc` against an in-process server (port 0): version, `command.run`, `eval.mesh`,
  `--port/--token`, wrong token, unknown method, bad `--params`, no editor (exit 1).
- `forge run`: a script that builds a scene with `--save` (the file loads and has the
  nodes); no `--save` leaves the scene file unchanged; `--scene` input; `--json` output;
  a syntax error, a runtime error and an operation-limit stop are exit 1 and save
  nothing; a missing script file is exit 1; a missing scene is exit 2.
- A closed stdout pipe (`forge --json commands | head -c 300`) is exit 0 without a panic:
  output goes through a locked stdout handle and `BrokenPipe` is a quiet success.
