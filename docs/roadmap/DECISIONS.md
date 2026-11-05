# Decisions the plan rests on

The planner made these decisions so the plan could run without a round of questions. Each
one is the default. The owner can overrule any of them by editing this file. If a decision
changes, the lead marks the affected briefs `rework` in [BACKLOG.md](BACKLOG.md).

| # | Decision | Why | Alternatives considered |
|---|---|---|---|
| D1 | **egui (+ egui_dock) stays the UI toolkit**, rendered through wgpu | Already in use: 4 batches of UI built and tested with egui_kittest. Immediate mode suits a data-driven attribute panel. Works on all three OS | Slint, iced: a rewrite for no gain |
| D2 | **wgpu** for the viewport and the GPU renderer (Vulkan/Metal/DX12; GL fallback) | One API for all three OS; the viewport already uses it | raw Vulkan (no macOS), OpenGL (deprecated on macOS) |
| D3 | **Every user action is a `Command`**: a named, typed, undoable operation with a JSON-schema argument description, registered like node types. UI, shortcuts, CLI, RPC, MCP and scripts all go through it | One implementation per action; agents and humans get the same features; the command list documents itself | Separate APIs per front end (drift, duplicated bugs) |
| D4 | **Automation surface** | Agents need a stable, documented protocol; the bridge we used for the reference app showed the value of an auth token and a localhost-only socket | gRPC (heavier), HTTP REST (no push events) |
| | ‣ `forge` CLI: headless; same binary as the editor, `forge-studio --headless` alias | | |
| | ‣ JSON-RPC 2.0 over a localhost TCP socket with a token, plus stdio for the CLI | | |
| | ‣ MCP server `forge-mcp`, mapping MCP tools onto RPC | | |
| D5 | **Embedded scripting: Rhai.** Python is reachable through a thin client library over RPC (`forge-py`, pure Python, no embedding) | Rhai is pure Rust and builds on every OS with no runtime to ship; Python embedding (pyo3 plus a bundled interpreter) is fragile across OSes. A Python client over RPC gives Python users the same reach | Embedded Python (v2 candidate), Lua |
| D6 | **Volumes: our own sparse grid** (`forge-volume`): a two-level hashed block grid (8³ leaves) holding f32 SDF or density. Builder, filters and meshers are our own. OpenVDB `.vdb` read/write comes from the published file format, implemented in Rust, later (brief 086) | OpenVDB is C++; linking it breaks easy cross-platform builds. Our needs (SDF of meshes, splines, points; booleans; smoothing; meshing) fit a small, testable crate | vdb-rs (read-only, partial), OpenVDB via FFI |
| D7 | **Rendering: own CPU path tracer first**, then a wgpu compute path tracer that shares the material model. Viewport: a rasterizer with PBR preview | Parity is measured by PSNR against reference renders (level C); a CPU tracer is easy to test deterministically | Embedding Cycles (Apache-2 but huge, C++), external renderer bridges (parked, see ../REDSHIFT.md) |
| D8 | **Dynamics: rapier3d** for rigid bodies and colliders; own code for soft bodies (position-based dynamics), particles and forces | rapier is pure Rust, deterministic with fixed steps and cross-platform | PhysX/Bullet via FFI |
| D9 | **Exchange formats** | Pure-Rust where possible; one C library only where nothing else exists, built from source with `cc`, no system dependency | Assimp (C++, large) |
| | ‣ glTF: `gltf` crate | | |
| | ‣ FBX import: `ufbx` (C, MIT, vendored via `cc`); FBX export later | | |
| | ‣ Alembic: our own Ogawa reader/writer, from the public spec | | |
| | ‣ USD: `.usda` text writer and reader first, binary `.usdc` later | | |
| | ‣ STL and PLY: own code | | |
| D10 | **Licensing: MIT OR Apache-2.0** for our code. Dependencies must be permissive (MIT, Apache, BSD, Zlib, MPL-2.0 allowed); no GPL in the shipped binary. `cargo-deny` enforces it in CI | Keeps the project free to distribute; the owner can relicense later | GPL |
| D11 | **Releases on all three OS** | Unsigned builds work for a personal tool; signing can be switched on later without code changes | – |
| | ‣ Linux: AppImage + tar.gz; Flatpak manifest in v1.1 | | |
| | ‣ macOS: universal2 `.app` in a `.dmg`, unsigned by default; signing and notarization run when secrets are set | | |
| | ‣ Windows: `.zip` + MSI via cargo-wix, unsigned by default | | |
| D12 | **Parity target: R25.010** with the levels A/B/C of [../PLAN.md](../PLAN.md). Features the reference app gained after R25 are out of v1 scope | That is the reference install the owner has | – |
| D13 | **Out of v1 scope:** | Each is a large subsystem with little effect on S1–S10 | – |
| | ‣ sculpting, hair, cloth | | |
| | ‣ the reference app's node-based scene system (2023+) | | |
| | ‣ Take system, render farm, team render | | |
| | ‣ the external renderer bridge | | |
| | ‣ BodyPaint-style 3D painting | | |
| | ‣ the sound effector | | |
| | ‣ Python effectors and tags (Rhai equivalents exist) | | |
| | They are listed in the area files as "v2" | | |
| D14 | **Units and handedness follow the reference app**: centimetres, left-handed Y-up display, H/P/B rotation order. These are already in place (TRANSFORM.md, SMOOTHING.md §4). Exporters convert to right-handed metres where the target format expects it | Familiarity and parity | – |
| D15 | **Agent roles may be any capable model.** Briefs are written for a careful mid-tier coding model (e.g. Sonnet) working alone, with the full gate as the safety net. The lead runs on a top-tier model | That's the setup that delivered briefs 03–10 | – |
| D16 | **Integration flow** | Minimises the owner's role to "merge when you like", or nothing at all if auto-merge is on | A long-lived `dev` branch (adds a merge step) |
| | ‣ observers push to `observations` | | |
| | ‣ implementers commit on a work branch | | |
| | ‣ the lead reviews, runs the gate, opens a PR to `main` and turns on auto-merge (CI must be green) | | |
| | ‣ the observer loop reads its queue from `main` | | |
| D17 | **Node editor canvas: our own, in `crates/ui` (`node_canvas`), not `egui-snarl`.** The canvas is generic over a `CanvasModel` trait (nodes, typed ports, links, link check, add menu); the expression editor (brief 109) is the first model, node materials (124) the second | `egui-snarl` 0.12 is MIT OR Apache-2.0 (fits `deny.toml`) but depends on `egui ^0.36`, and the workspace is on egui 0.37, so it does not resolve (the same situation as `egui_dock`, brief 147). Its API also owns the graph (`Snarl<T>`) and applies edits itself, while here every edit must be a registry command with one undo step per drag, and node positions live in the graph parameter. A canvas is about 600 lines of drawing and hit-testing, all pure and unit-tested | `egui-snarl` pinned to an older egui (no), forking it (maintenance), `egui_node_graph` (unmaintained) |
| D18 | **R20 / R25 conflicts: R25 wins (follows D12).** When golden cases recorded from R20 docs and from the R25 app disagree for the same inputs, generators follow R25. The R20 case moves to its test's KNOWN_MISMATCH with the reason `superseded by R25 (D18): <r25 case>`; no version switch is added to node parameters | D12 makes R25.010 the target; a hidden version flag would double every rule and its tests for behaviour no user of v1 can select | A `compat_version` parameter per generator (rejected); deleting the R20 golden data (rejected: it still pins everything R20 and R25 share) |
