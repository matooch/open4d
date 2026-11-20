# 090 · Dope sheet manager

Status: done (parity-pending)

## Goal
A dope sheet with the track tree, key selection, move/scale/ripple, copy/paste, all through registry commands.

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)
- GUI session G5 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G5

## Design decisions (planner)
- `TimeView` (pan/zoom/ruler/snapping) and `KeySelection` shared with the F-curve view. Pure logic in `fcurve_logic.rs` (hit tests, box select, scale about pivot).
- Every edit is a command (`key.move`, `key.scale`, `key.delete`, …) so RPC and undo come for free.

## Done when
- [ ] egui_kittest: box select + move + undo.
- [ ] Unit tests for the pure logic.
- [ ] Commands callable through `forge` CLI.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `crates/ui/src/{dopesheet,fcurve_logic}.rs`, `crates/anim/src/keyedit.rs`, `crates/command/src/builtin/keys.rs` (key.move, key.scale, key.delete, key.set_interp, key.paste, track.set_behavior, key.euler_filter, key.set_tangent), Dope Sheet panel in the dock. Tests: `crates/ui/tests/key_edit.rs` (box select + move + undo and click + paste under egui_kittest, command and pure-logic tests). No parity data: own design, parity-pending G5.
- Rules: ANIM.md §1b. Moves are whole frames when snap is on; ripple moves the selected key and all later keys of the track.
- Deviations: the Window menu's per-panel "Move X" entries are now under one "Move panel" submenu (16 panels overflowed the menu and hid "Reset layout"); `tests/ui.rs` clicks it first. Panels are registered as dock panels, not opened by default. Copy/paste clipboard is panel state (paste is a stateless `key.paste`).
- Files: crates/ui/src/{dopesheet,fcurve_logic,dock,app,lib,icon_table}.rs, crates/ui/tests/{key_edit,ui}.rs, crates/command/src/builtin/{keys,mod}.rs, crates/command/Cargo.toml, crates/anim/src/keyedit.rs, docs/manual/reference/*, docs/specs/ANIM.md.
- Open: G5 answers for the reference's gestures, modifiers, ripple and summary-row behaviour.
- Gate: see 091.
