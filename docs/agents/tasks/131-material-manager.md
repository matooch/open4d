# 131 · Material manager UI with preview renders

Status: done (parity-pending: G7) · Area: J · Depends on: 123, 136 · Observations: G7 · Touches: ui

## Goal
Material manager with thumbnails rendered by the CPU path tracer; drag onto objects.

## Inputs
- Area design: [../../roadmap/areas/J-materials.md](../../roadmap/areas/J-materials.md)
- GUI session G7 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G7 §1–2

## Design decisions (planner)
- Thumbnails rendered in a background job at low sample count.

## Done when
- [x] egui_kittest: drag a material onto an object creates a material tag.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- The Materials panel displays a grid of CPU-rendered studio previews. One
  background job runs at a time; the icon colour remains visible while loading,
  and material or linked-shader changes invalidate the thumbnail.
- A pointer drag onto a named object in the Object Manager adds one material
  tag without changing selection; the test verifies undo and leaves other
  objects unchanged. The viewport also accepts the typed payload via picking
  and uses the same assignment command.
- The fixed-seed 64×64 thumbnail test verifies repeatability and a red-to-blue
  material edit. G7 §§1–2 are unobserved; thumbnail scene, layout and drag
  interaction are native designs, not reference parity.
- Files: MATERIAL §M7, UI `material_manager`, Object Manager and viewport
  drop targets, and `crates/ui/tests/material_manager.rs`.
- Gate: all five material and 26 Object Manager tests passed. `GATE: 2487
  tests passed, 0 failed`; `GATE: all passed`.
