# Observer playbook (MCP bridge)

You observe the reference app (R25, macOS) as a **black box** and turn what you see into
observation files (`docs/parity/<feature>.md`) and golden cases (`tests/golden/`). You
drive the app through the MCP bridge the owner installed (a socket plugin inside the app
plus an MCP server), falling back to the GUI only when a step can't be scripted. Read
[CLEANROOM.md](../CLEANROOM.md), [PARITY.md](../PARITY.md) and
[parity/README.md](../parity/README.md) first; this file adds the MCP rules.

## 0. Clean-room rules for the bridge

The bridge can do far more than we may use. These rules decide what you call.

**Allowed tools** (black-box use: build a scene, set values, read results back):
`ping`, `doctor`, `get_capabilities` (version only), `get_c4d_info` (version only),
`execute_python_script`, `add_primitive`, `get_parameter`, `set_parameter`,
`modify_object`, `list_objects`, `get_object_info`, `dump_object_tree`,
`scene_snapshot`, `scene_diff`, `set_keyframe`, `run_modeling_command`, `apply_deformer`,
`add_field_to_scene`, `create_mograph_cloner`, `add_effector`, `viewport_screenshot`,
`render_frame`, `render_preview`, `image_inspect`, `images_compare`, `undo`, `redo`,
`begin_undo_group`, `end_undo_group`, `save_scene`/`load_scene` (local scratch only),
`get_console_log`, `clear_console_log`.

**Forbidden tools and files** (they read the app's internals or install into it):
every `scene_nodes_*` tool, the MCP repo's `data/*.json` atlas files, `cpp_shim/`
(compiles against SDK headers), `list_installed_plugins`, `install_plugin`,
`build_and_install_plugin`, `enumerate_octane_plugins`, `tap_octane_log`, anything
that opens the app's install, preferences or library folders.

**Allowed with care:** `enumerate_descids` / `find_command_by_name` to *find* the
parameter you want to set. What they return stays on your machine: the repo only gets
the **label the Attribute Manager shows** and the value.

**Probe scripts stay outside the repo.** The Python you send through
`execute_python_script` uses the app's public scripting API, so it contains the app's
module and symbol names; the cleanroom gate rejects those. Keep scripts in your observer
workspace (`~/forge-observer/probes/<feature>.py`); commit only their **outputs**
(`mesh.obj`, `samples.json`, `case.toml`, the observation file). Describe in the
observation file *what* a probe set and read, in plain words.

**Never commit:** app scene files, screenshots, renders of the app's UI, preset or
library content, parameter IDs or symbol names.

Check the app's EULA allows scripted observation before the first run (§1 step 6); stop if not.

## 1. Setup on the Mac (once, done by the owner, not by a session)

Keep observer and implementer separate even though they share a machine: the
implementer must never be able to reach the app.

**Security first.** The bridge's plugin listens on **all network interfaces**
(`0.0.0.0:5555`) and `execute_python_script` runs arbitrary code on the Mac. Without a
token, anyone on your network can run commands. The plugin reads `MCP_AUTH_TOKEN` from
**the app's own environment** when the app starts, so the token must reach the app, not
only the MCP server.

1. **Token into the app's environment.** Apps started from the Dock don't see shell
   variables. Make a token once, put it in your shell profile, and hand it to the app
   via `launchctl` (lasts until reboot; repeat after a restart, or add it to a login
   item):
   ```bash
   echo "export MCP_AUTH_TOKEN=$(openssl rand -hex 24)" >> ~/.zshrc && source ~/.zshrc
   launchctl setenv MCP_AUTH_TOKEN "$MCP_AUTH_TOKEN"
   ```
   Quit and restart the app. Its console should log
   `MCP_AUTH_TOKEN is set — commands must include auth_token field`. Allow the
   incoming connection only if macOS asks; you can also deny it in the firewall
   (the bridge only needs localhost).
2. **Plugin loaded?** With the app running:
   `lsof -nP -iTCP:5555 -sTCP:LISTEN` must show the app. If not, the plugin
   isn't in the R25 plugins folder or failed to load on R25 (Python 3.9): check the
   app's Python console for the error and report it.
3. **MCP server** (needs Python ≥ 3.10 on the Mac, separate from the app's Python):
   ```bash
   cd ~/Github/cinema4d-mcp && python3 -m venv .venv && .venv/bin/pip install -e .
   ```
4. **Register it only in the observer workspace.** Create
   `~/forge-observer/.mcp.json` (the token is expanded from your shell, never written
   into the file):
   ```json
   {
     "mcpServers": {
       "refapp": {
         "command": "/Users/<you>/Github/cinema4d-mcp/.venv/bin/cinema4d-mcp-wrapper",
         "args": [],
         "env": {
           "MCP_AUTH_TOKEN": "${MCP_AUTH_TOKEN}",
           "C4D_HOST": "127.0.0.1",
           "C4D_PORT": "5555"
         }
       }
     }
   }
   ```
   Leave `MCP_SAFE_MODE` unset (Python execution needs it off).
5. **Start observer sessions** from that folder with the repo added, and approve the
   project server when asked:
   ```bash
   cd ~/forge-observer && claude --add-dir ~/Github/open4d
   ```
   `/mcp` must list `refapp` as connected. Implementer sessions start from the repo
   folder; `/mcp` there must **not** list it.
6. **EULA.** The owner confirms the reference app's licence allows scripted use through
   its own Python API before O0 runs, and notes it in r25-drift.md §0.

The repo's paths are relative to the repo (`docs/parity/…`), wherever the clone lives.

## 2. O0 · Smoke test (do this before any other observation)

The bridge's README lists R25 as unsupported (it targets 2024+). Find out what works:

1. `ping`, then `get_capabilities`: record the app version string and the plugin
   version in [parity/r25-drift.md](../parity/r25-drift.md) §0.
2. `execute_python_script` with a probe that returns the app version and `1 + 1`. If it
   fails, record the error text and stop: MCP observation is off, use the GUI method
   (parity/README.md) and tell the planner.
3. A probe that makes a new document, adds a default cube, and exports it as Wavefront
   OBJ with the settings in [parity/export-settings.md](../parity/export-settings.md)
   (scale 1 cm, normals Vertices, UVs on, No Material, **Flip Z off**, everything else
   default) to a scratch path. Compare it with `tests/golden/cube/defaults/mesh.obj`
   (`cmp`, then `golden info` on both). Record the R25 exporter's option list in
   r25-drift.md §1: new options, changed defaults, and how you set them from the probe.
4. A probe that sets a key on a cube's position X at frame 0 and 30 and reads back the
   evaluated value at frame 15. Record whether this works (O2 needs it).

Write down which tools failed and with what message. The planner adjusts the forms.

## 3. The loop per case (scripted)

1. New, empty document (`execute_python_script`).
2. Build exactly what the case row says; set only the listed parameters.
3. Read back every parameter you set (`get_parameter` or the probe) and write the value
   the app **shows** into the case row. Clamped values are findings.
4. Export or sample:
   - Mesh cases: OBJ export as above → `tests/golden/<feature>/<case>/mesh.obj`.
   - Numeric cases (keys, fields, clones): the probe prints JSON; save it as
     `tests/golden/<feature>/<case>/samples.json` in the format the form gives.
5. Write `case.toml` (parity/README.md §2); run `golden info` (mesh cases) and paste the
   output into the row.
6. When the feature is done: `cargo run -q -p forge-parity-harness --bin golden -- check`
   prints `ok: N cases`. Commit locally on the batch branch, one commit per feature:
   `Observation: <feature> (N cases)`.

A probe that leaves the document dirty can leak into the next case: always start from a
new document, and close it after the export.

## 4. When to use the GUI instead

Defaults and ranges as the Attribute Manager shows them, menu paths, which parameters
hide others, tool behaviour, and anything about layout or look. Use screenshots to
understand; never commit them. The GUI doctrine in [../../MEMORY.md](../../MEMORY.md)
(Esc before selecting, crop-verify focus before typing) still applies.

## 5. Hand-back

Report to the planner: forms filled, cases exported, failures with their error text,
anything surprising (clamping, hidden parameters, version differences).
