# 152 · Key map: defaults, customisation editor

Status: done (parity-pending G2) · Area: L · Depends on: 011 · Observations: G2 · Touches: ui

## Goal
A KeyMap from chords to command ids, defaults from G2, user overrides, and an editor.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)
- GUI session G2 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G2 §1

## Design decisions (planner)
- Defaults in a TOML we write; overrides in the settings dir (175). Cmd on macOS, Ctrl elsewhere. Conflicts reported in the editor.

## Done when
- [x] Unit tests for chord parsing and conflicts; egui_kittest: shortcut runs a command.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 22 tests in `crates/ui/tests/keymap.rs` (chord round trip over every egui key and
  modifier combination, defaults load / no conflicts / every bound command exists and its args
  fit the command schema, conflict rules, override merge and inverse, settings file, editor
  rows, kittest: default shortcuts run commands and Ctrl+Z undoes, rebind / conflict / clear /
  reset in the editor window) and 1 in `crates/command/tests/registry.rs`. No parity cases:
  G2 is unanswered, so every default binding is **parity-pending G2** (own design, UI.md §14.2).
  Gate: 902 tests passed.
- Rules: `KeyMap` of `Binding {chord, command, args, scope}`; defaults in
  `crates/ui/keymap/default.toml`; user overrides are the `keymap` object of the settings
  file (`forge_rpc::settings_path()`, chord -> binding or null); exact modifier matching, one
  command per key press; conflicts = same chord with overlapping scope, Replace/Cancel in the
  editor (UI.md §14.3-14.5). `Mod` is egui `Modifiers::COMMAND`.
- Deviations: (1) the old hard-coded shortcuts were not all registry commands, so seven
  commands were added (`file.open_dialog`, `file.save_dialog`, `file.save_as_dialog`,
  `object.begin_rename`, `view.gizmo_mode`, `view.gizmo_space`, `timeline.toggle_play`);
  window actions go through `CommandContext::request_ui`, headless contexts refuse, RPC
  refuses dialogs. Gizmo mode/space moved from `Editor` into `EditorState`. (2) The editor is
  a floating window (Window > Key map...), not a dock panel, so the layout format is unchanged.
  (3) New default keys beyond the old ones: Mod+I, Alt+1..4, Mod+Shift+E. (4) Menus show the
  chord from the key map; Undo/Redo/Delete show none (as before, keeps their labels exact).
- Files: crates/ui/{keymap/default.toml, src/keymap.rs, src/keymap_editor.rs, src/app.rs,
  src/state.rs, src/commands.rs, src/lib.rs, Cargo.toml, tests/keymap.rs}, crates/command/
  {src/builtin/{file,object,timeline}.rs, src/context.rs, src/state.rs, src/lib.rs,
  tests/registry.rs}, docs/specs/{UI,COMMANDS}.md, docs/manual/reference/*, BACKLOG.md.
- Existing UI tests changed: none.
- Open questions: G2 (the real default keys, mouse navigation); whether settings should
  move to a TOML file with brief 155; Escape cannot be bound (it cancels capture).
- Gate: `GATE: 902 tests passed, 0 failed` ... `cleanroom: ok` ... `GATE: all passed`.
