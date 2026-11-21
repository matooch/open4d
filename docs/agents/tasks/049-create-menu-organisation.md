# 049 · Create menu organisation

Status: ready · Area: C · Depends on: 148 · Observations: G1 · Touches: ui, core

## Goal
The Create menu groups and orders object types like the reference: Object, Spline, Generator, Deformer, Field, Clone toolset, Volume, Camera, Light, Tag.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- GUI session G1 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS G1 answers
- crates/ui/src/commands.rs create_menu

## Design decisions (planner)
- `NodeType` gains `menu_path` and `menu_order` (registry metadata). Icons come from 148.

## Done when
- [x] UI.md updated; a UI test checks the menu structure.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

Status: done

Added `menu_path: Option<String>` and `menu_order: i32` to `NodeType` with a
`menu_path(path, order)` builder. Updated primitive, spline primitive, generator,
volume and scene-object registrations to set their menu paths. `create_menu` now
returns groups by `menu_path` in the reference order (Object, Spline, Generator,
Deformer, Field, Volume, Camera, Light, Tag), sorting entries by `menu_order` then
label. Categories without an explicit path derive a default group (Deformer, Field,
Tag). The Clone toolset and Scene (Environment/Floor/Sky) submenus remain separate.

Updated `crates/ui/tests/commands.rs` with `create_menu_is_grouped_like_the_reference`,
and adjusted existing tests in `editing.rs` and `scene_overlay.rs` to the new group
labels. Updated `docs/specs/UI.md` §16.2 and appended a row to
`docs/parity/STATUS.md`. BACKLOG.md row 049 set to `done`.

Parity: own design until GUI session G1 is answered; grouping and wording marked
parity-pending.

Co-Authored-By: kimi-k2.7-code <kimi-k2.7-code@nousresearch.com>
