# 154 · Content browser (own presets) and recent files

Status: done · Area: L · Depends on: 163 · Observations: – · Touches: ui

## Goal
Browse our own preset library and recent files.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)

## Design decisions (planner)
- Presets are native files we author; never the reference presets.

## Done when
- [x] Load a preset from the browser.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: no parity data (own design, no observation). Tests added: 4 unit tests in
  `crates/command/src/browser.rs`, 14 in `crates/app/tests/browser.rs` (1 ignored generator for
  the preset files), 9 egui_kittest in `crates/ui/tests/content_browser.rs`.
- Rules (docs/specs/UI.md §26, COMMANDS.md §6): presets are native `.forge` scenes (3 built-in
  ones written for this project, plus user files in `<data>/presets/objects`); `browser.load`
  grafts the roots under a parent in one undo step, remapping links inside the preset and
  dropping others and the layer link; recent files live in the settings key `recent.files`,
  max 10, recorded by `file.open/save/save_as`. All of it is parity-pending (panel layout,
  tab names, folder structure, recent count).
- Deviations: the materials kind is not implemented, because no material node type exists
  yet (area J); the folder layout and `kind` argument leave room for it. `FS_COMMANDS` of the
  fuzz harness gained the four `browser.*` commands that touch files, and the generated
  `command_args` corpus was regenerated (its file index prefix shifts when ids are added).
- Files changed: `crates/command/src/{browser.rs,builtin/browser.rs,builtin/file.rs,
  builtin/mod.rs,context.rs,lib.rs}`, `crates/command/presets/objects/*.forge`,
  `crates/app/{src/lib.rs,tests/browser.rs}`, `crates/platform/src/lib.rs`,
  `crates/ui/src/{content_browser.rs,dock.rs,app.rs,app/files.rs,state.rs,commands.rs,
  icon_table.rs,lib.rs}`, `crates/ui/tests/content_browser.rs`,
  `crates/fuzz-harness/src/targets.rs`, fuzz corpus, specs (UI §26, COMMANDS, PLATFORM),
  generated `docs/manual/reference`, BACKLOG, L-ui checklist.
- Open questions: the reference layout of a content browser (tabs, folders, thumbnails) and
  how many recent files it keeps (a GUI observation session would settle both).
- Gate: `GATE: 1715 tests passed, 0 failed` ... `GATE: all passed`.
