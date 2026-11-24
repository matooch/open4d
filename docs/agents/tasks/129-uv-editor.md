# 129 · UV editor (view, transform, relax LSCM, pack, seams)

Status: done (parity-pending: G7) · Area: J · Depends on: 128, 150 · Observations: G7 · Touches: ui, geometry

## Goal
An interactive UV editor with relax (LSCM) and pack.

## Inputs
- Area design: [../../roadmap/areas/J-materials.md](../../roadmap/areas/J-materials.md)
- GUI session G7 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G7 §4

## Design decisions (planner)
- LSCM via nalgebra-sparse; skyline packing; all ops are commands.

## Done when
- [x] Unit tests: LSCM of a cut cylinder is near-isometric; packing has no overlaps.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- UV workspace displays the active polygon's corner UVs, with vertex selection,
  drag, scale, rotation, seam marking, relax and pack. Edits are registry commands
  and write the existing/new UVW tag in one undo step.
- LSCM assembles area-weighted conformal rows with `nalgebra-sparse` and uses a
  preconditioned least-squares iteration. The cut cylinder has relative edge
  stretch below 1.00001 after accounting for uniform scale. Skyline packing
  preserves relative scale and orientation; tests check padding and zero overlap.
- Application tests exercise transform/undo, seam storage, relax and pack; the
  egui test opens the UV workspace and runs relax/pack on the active polygon.
- G7 §4 is unobserved. Layout/interaction is native; closed charts require a seam.
  Marked seam edge indices are for the current topology, not remapped by modeling edits.
- Files: UV §U7, geometry `uv_edit`, App `uv_commands`, UVW tag, UV Editor/workspace,
  and the geometry, application and egui tests.
- Gate: `GATE: 2497 tests passed, 0 failed`; `GATE: all passed`.
