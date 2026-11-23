# 097 · Keyframe indicators on every parameter; key context menu

Status: done (parity-pending: G4) · Area: G · Depends on: 088, 150 · Observations: G4 · Touches: ui

## Goal
Every animatable parameter shows a key indicator (none / animated / key on this frame / changed) and a context menu (add, delete, next/previous key).

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)
- GUI session G4 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G4 §2

## Design decisions (planner)
- Generic: derived from ParamDesc, no per-node code.

## Done when
- [x] egui_kittest: click the indicator to key, value change shows the "changed" state.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- **Native coverage:** indicator click, staged Bool edit, key update and single undo
  step, row-only jump and deletion, and three-channel vector keying/undo. Both test
  suites passed (22 animation tests, 19 Attribute Manager tests).
- **Design:** marker and channels follow the descriptor's supported track kinds.
  `key.set_parameter` preserves existing key time/interpolation and adds all row
  channels in one step; user-data descriptors also resolve in animation evaluation.
  When Auto-Key is off, single-object edits to animated Attributes rows are local
  drafts until keyed. Scripted edits, multi-edit, and gizmo/Coordinates semantics
  are unchanged.
- **Files:** UI and ANIM specs; descriptor key command and reference; animation
  descriptor lookup; Attributes marker, widgets, editor and kittest.
- **Parity pending G4:** need real indicator state colours and controls; confirm
  whether pending edits survive leaving the frame or are shown in the viewport.
- **Gate:** `GATE: 2452 tests passed, 0 failed`; `GATE: all passed`.
