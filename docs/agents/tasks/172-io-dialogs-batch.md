# 172 · Import/export dialogs and CLI batch export

Status: done · Area: M · Depends on: 013, 150 · Observations: – · Touches: ui, io

## Goal
Import/export dialogs with options, and `forge export` batch conversion.

## Inputs
- Area design: [../../roadmap/areas/M-io.md](../../roadmap/areas/M-io.md)

## Design decisions (planner)
- Options are ParamDescs so dialogs derive.

## Done when
- [x] CLI converts a folder of files.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 6 io, 6 app, 7 CLI and 5 UI tests added (plus script/fuzz list updates). No parity
  cases (own design; no observation exists).
- Rules derived: option descriptors per format (`forge_io::formats`: `flip_z` on OBJ import,
  `normals` and `weld` on OBJ export, defaults equal the struct defaults); generic commands
  `file.import`, `file.export`, `io.formats`, `file.import_dialog`, `file.export_dialog`;
  options window built from descriptors; `forge convert` (folder or file, `--recursive`,
  `--force`, `--option`, `--format obj|forge`). Parity-pending: dialog layout, titles, option
  order and names (IO.md section 3, UI.md section 27).
- Deviations: none from the design decisions. `file.import_obj`/`file.export_obj` stay as
  aliases with default options; the old `Editor::import_obj`/`export_obj` stay immediate.
  `forge export` now accepts `--option`. Only OBJ exists, so the folder conversion is scene to
  OBJ and OBJ to native scene.
- Files changed: docs/specs (IO, CLI, COMMANDS, UI, PLATFORM, SCRIPTING), docs/manual
  (automation, generated reference), crates/io (formats.rs), crates/command (file.rs, state.rs),
  crates/app, crates/ui (app.rs, app/files.rs), crates/script, crates/fuzz-harness,
  apps/forge (convert.rs, lib.rs), tests listed above, roadmap rows.
- Open questions: dialog layout and option names of the reference app (needs an observation);
  more formats (briefs 164 to 171) plug into `formats.rs`.
- Gate: see the commit message.
