# 150 · Attribute manager parity: tabs, widgets, multi-edit, keyframe dots, history

Status: done (parity-pending G4) · Area: L · Depends on: 147 · Observations: G4 · Touches: ui

## Goal
Attribute manager driven by ParamDesc with tabs/groups, all widget types, multi-object editing and scrubbing.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)
- GUI session G4 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G4

## Design decisions (planner)
- One widget module per type (float, int, bool, enum, vector, colour, link, spline curve, gradient, field list, link list). Scrub modifiers per G4 (parity-pending).
- Multi-edit shows mixed values and sets all.

## Done when
- [x] egui_kittest per widget type; multi-edit test.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `crates/ui/tests/attribute_manager.rs` (16 tests): egui_kittest edit -> command -> value -> undo for float (display units), int, bool, enum, vector (one component), string (typing merges into one step), link (pick, clear), link list (add, move, remove); colour is driven through `param.edit` (the picker popup cannot be driven by kittest) and its row is shown; tabs/groups; multi-edit with mixed values, one undo step for all selected, vec3 component keeps the others; common-parameter intersection and skipping; gesture merge and autokey; scrub maths and base step as pure unit tests. No parity numbers (no golden data).
- Rules (all own design, parity-pending G4): group key `"Tab/Group"`, default tab `Basic`; scrub step = `UiHint.step` else 1 (int) / 0.1 (angle, percent) / 0.01; Shift = x0.1, Ctrl = x10, other combinations (incl. Alt) = x1; Up/Down arrows in a focused field add one step (egui); multi-edit rows = active object's params present with equal kind in all selected; mixed shown as "mixed" / indeterminate / "(mixed)". Spec: docs/specs/UI.md §12, COMMANDS.md §6 (`param.edit`).
- Deviations: (1) New command `param.edit` (registered in forge-app, because it needs the animation rules) instead of `param.set`, which is single-object, non-animated and not mergeable; `forge-command` now exports `param_value_from_json` and `display_to_stored`. (2) No spline-curve, gradient or field-list widget: those parameter kinds do not exist yet (add with the kind). (3) Lock/unlock and history back/forward from the area list are not done. (4) Keyframe dots: only the existing marker slot is kept and documented as 097's hook. (5) The tool settings panel (`tool_ui`, brief 020) still has its own widgets; moving it onto `widgets` is a follow-up. (6) Float rows are not clamped in the field (the document clamps, as before). (7) `apply_edit`/`apply_edit_anim` remain for tests and callers; the panel no longer uses them.
- UI tests changed: none (all existing tests pass unchanged; `view.groups` kept its shape).
- Files: crates/app/{Cargo.toml,src/lib.rs,src/param_edit.rs}, crates/command/src/{lib.rs,builtin/mod.rs,builtin/param.rs}, crates/ui/src/{app.rs,attributes.rs,lib.rs,widgets/*}, crates/ui/tests/attribute_manager.rs, docs/specs/{UI,COMMANDS}.md, docs/manual/reference/{commands,mcp}.md, docs/roadmap/BACKLOG.md, this brief, Cargo.lock.
- Open questions (G4): scrub modifiers and arrow clicks, key dot colours and click action, field-list columns, link drop behaviour.
- Gate: `GATE: 853 tests passed, 0 failed` ... `cleanroom: ok` ... `GATE: all passed`.
