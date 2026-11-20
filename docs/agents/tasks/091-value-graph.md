# 091 · Value graph (F-curve) manager

Status: done (parity-pending)

## Goal
The F-curve editor: curves per channel, handles, tangent editing, normalised view, frame all/selected.

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)
- GUI session G5 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- O38: [../../parity/fcurves.md](../../parity/fcurves.md) and its golden cases
- GUI-SESSIONS.md G5 (handle look and behaviour)

## Design decisions (planner)
- Curves drawn by sampling the evaluator (the same code as playback). Handle drag edits tangent through `key.set_tangent`; Shift/modifier behaviour from G5 (parity-pending until answered).

## Done when
- [ ] egui_kittest: drag a handle, the curve and the evaluated value change, undo restores.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `crates/ui/src/fcurve.rs` (Value Graph panel): curves sampled from the evaluator, axis colours, keys and evaluated handles (`forge_anim::handles`), handle drag -> `key.set_tangent`, key drag -> `key.move`, box select, frame all / selected, normalised view, tangent presets, break / unify / weighted, numeric key entry. Test: `value_graph_handle_drag_changes_the_curve_and_undo_restores_it` (egui_kittest) plus the command tests of 090.
- Rules: ANIM.md §1b; own design, parity-pending G5 (modifier gestures) and O38 (handle semantics are the §1.5a stored-handle rule).
- Deviations: in the normalised view value and handle drags are disabled; multi-key tangent presets send one command per key (one undo step each).
- Files: as 090 plus crates/ui/src/fcurve.rs.
- Open: G5 handle gestures; a single-step multi-key `key.set_tangent`.
- Gate: GATE_PLACEHOLDER
