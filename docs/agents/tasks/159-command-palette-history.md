# 159 · Command palette and undo history panel

Status: done · Area: L · Depends on: 011 · Observations: – · Touches: ui

## Goal
Fuzzy search over all commands; a panel listing undo history with jump-to.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)

## Design decisions (planner)
- Reads the command registry; no hard-coded list.

## Done when
- [x] egui_kittest: run a command from the palette.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: spec `docs/specs/UI.md` §18 (+ §14.2 row, `COMMANDS.md` §6). Tests added: 12 unit in `forge_ui::palette` (fuzzy matcher orderings, entries from the registry and key map, search by id/label/arguments/chord, recency order, recent bookkeeping, schema forms and `build_args`), 3 unit in `history_panel` (rows, current mark), 1 in `forge-command/tests/registry.rs` (`edit.history_jump` back/forth/range), 14 egui_kittest in `crates/ui/tests/palette.rs` (open/close, run by Enter and click, preset with arguments, arrows, form with required field, bad value, enum form, recent first, shortcuts blocked while open, history panel closed by default and opened from the Window menu, rows follow edits, jump by click with undo/redo, merged gesture is one row). No parity cases (own design).
- Rules: palette = fuzzy subsequence score (DP over alignments, word-start / run / prefix bonuses), every query token must match one of id, label, arguments, chord (canonical and platform spelling); empty query lists recent commands first, then by label. A command needs a form when `{}` fails its `args_schema()`; otherwise it runs directly. One extra "preset" entry per key binding with arguments (e.g. Shading: mode=wire) so every chord of the key map is listed.
- New commands: `ui.command_palette` (window action, Mod+Shift+P, no collision with the defaults), `edit.history_jump {position}` (repeated undo/redo; the history panel's click). New panel `Panel::History` (default area right bottom, closed by default; `Dock::default()` unchanged). Window menu: "History panel", "Command palette...".
- Deviations: recent commands are in memory only (not saved to the settings file); the history panel jumps with one command `edit.history_jump` rather than N clicks of `edit.undo`, so a jump is one registry call; the palette opens as a floating window, not a dock panel.
- Files changed: crates/command/src/{state.rs,builtin/file.rs,builtin/object.rs}, crates/command/tests/registry.rs, crates/ui/src/{palette.rs,history_panel.rs,app.rs,dock.rs,lib.rs}, crates/ui/keymap/default.toml, crates/ui/tests/palette.rs, docs/specs/{UI,COMMANDS}.md, docs/manual/reference/{commands,mcp}.md, BACKLOG, this brief.
- Open questions: should recent commands persist across sessions (settings file `palette.recent`)? Parity-pending: the Mod+Shift+P binding (GUI session G2).
- Gate: see the commit message / final report.
