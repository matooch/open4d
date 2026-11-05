# 156 · Layout save/load and workspaces

Status: done (parity-pending G1) · Area: L · Depends on: 147 · Observations: G1 · Touches: ui

## Goal
Save/load layouts and switch workspaces (Standard, Modeling, Animate, UV, Rigging).

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)
- GUI session G1 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**

## Design decisions (planner)
- Layouts are our own serialised files.

## Done when
- [x] Round-trip test.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: unit tests in `crates/ui/src/workspaces.rs` (8: names, every workspace valid with Viewport and a JSON round trip, workspaces differ and use the newer panels, name checks, save/load/list/delete round trip in a temporary folder, no folder, broken/old/newer/doubled/empty files fall back with a warning, last layout); `crates/ui/tests/workspaces.rs` (7 egui_kittest: each workspace by command with the document, undo history and selection untouched, Node workspace shows the node editor, Workspace menu switch and reset, save/load/delete by command and menu listing, bad names, broken and old files give the standard layout plus a status warning, last-layout restore incl. a broken file, the "Save layout as" window); `crates/command/tests/registry.rs` (headless refusal, not undoable, argument checks). All existing layout tests unchanged and green. No parity numbers.
- Rules (own design, parity-pending G1): UI.md §21, PLATFORM.md §8, COMMANDS.md §6. Workspaces Standard, Modeling, Animate, UV, Rigging, Node as tables of panels and sizes; layouts in `<config>/layouts/<name>.json` (+ `_last.json` restored at start, written at exit); `Layout.workspace` (serde default, version stays 1); `Layout::load_text` falls back to the standard layout with a warning for broken, older (<1) or newer (>1) files; commands `ui.workspace`, `ui.layout_save`, `ui.layout_load`, `ui.layout_delete` as window requests (headless: `needs the editor window`), menu bar "Workspace" menu.
- Deviations: (1) `UiRequest` lost `Copy` and gained four variants with a name. (2) Window-side errors (unknown name, I/O) go to the status bar, not to the command result, as with the other window requests. (3) The UV workspace has no UV panel yet (brief 129); it is a viewport-centred arrangement. (4) `LAYOUT_VERSION` stays 1: the new `workspace` field is an additive serde default, so existing files and tests stay valid; the migration path is the lenient loader. (5) Resizing: a new id salt (`layout_gen`) makes egui re-seed panel sizes on every `set_layout`. (6) `serve_ui_requests` also runs at the start of each frame so commands run by scripts/tests are served. (7) `tempfile` added as a dev-dependency of forge-ui.
- Files: crates/ui/{Cargo.toml,src/{app.rs,dock.rs,layout.rs,lib.rs,workspaces.rs},tests/workspaces.rs}, crates/platform/src/lib.rs, crates/command/{src/state.rs,src/builtin/file.rs,tests/registry.rs}, docs/specs/{UI,PLATFORM,COMMANDS}.md, docs/manual/reference/{commands,mcp}.md, docs/roadmap/BACKLOG.md, this brief, Cargo.lock.
- Open questions (G1): the reference's workspace names and contents, whether layouts are per-window or global, where the switcher sits.
- Gate: `GATE: 1553 tests passed, 0 failed` ... `cleanroom: ok` ... `GATE: all passed`.
