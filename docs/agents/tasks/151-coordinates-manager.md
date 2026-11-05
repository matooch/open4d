# 151 · Coordinates manager modes

Status: done (parity-pending G1) · Area: L · Depends on: 150 · Observations: G1 · Touches: ui

## Goal
Coordinates manager with object/world, size/scale modes and apply.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)
- GUI session G1 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G1

## Design decisions (planner)
- Values through transform commands.

## Done when
- [x] Tests for each mode.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `crates/ui/tests/coordinates_manager.rs` (10 tests: mixed detection, typed axis on two objects as one undo step, world space under a parent, Size row, Manual draft + Apply as one step, Reset, draft arguments, component rows via `mesh.transform` (pure and through the panel, no duplicate object rows)); `crates/app/tests/transform_edit.rs` (7 tests: null axes, degrees, world, size, one step and gesture merge, errors change nothing, `mesh.transform` gesture merge). No parity numbers (no golden data).
- Rules (all own design, parity-pending G1): UI.md §20. Space Local|World, third row Scale|Size, `Manual` check box with Apply and Reset (Immediate by default), `mixed` axes for several objects, a typed axis is set on all selected objects and keeps each object's other axes, component rows (Pivot, Rotation, Size from brief 034) replace the object rows while components are selected and share Local|World and Manual/Apply.
- Deviations: (1) The panel runs the new `transform.edit` (forge-app, `coords.rs`) rather than `transform.set`: `transform.set` is single-object, local, not animation-aware and not mergeable, so `transform.edit` is its animation-aware sibling as `param.edit` is of `param.set` (`transform.set` stays for scripts). (2) `mesh.transform` got an optional `gesture` so a dragged component field is one undo step. (3) The coordinate maths moved from `forge_ui::coordinates` to `forge_app::coords` (re-exported, `set_coord*` unchanged). (4) `ComponentDrag::apply_entry`/`CoordEntry` are kept for tests but the panel no longer uses them. (5) Rotation rows of an object use degrees, like before; "Object (Abs)" and "Size+" still not implemented.
- Files: crates/app/{src/coords.rs,src/lib.rs,tests/transform_edit.rs}, crates/command/src/builtin/components.rs, crates/ui/{src/app.rs,src/coordinates.rs,src/component_edit.rs,src/manipulate.rs,tests/coordinates_manager.rs}, docs/specs/{UI,MANIPULATE,COMMANDS}.md, docs/manual/reference/{commands,mcp}.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions (G1): the reference's Apply semantics (does Enter apply?), mode names, whether Size+ and absolute object coordinates are wanted.
- Gate: `GATE: 1537 tests passed, 0 failed` ... `cleanroom: ok` ... `GATE: all passed`.
