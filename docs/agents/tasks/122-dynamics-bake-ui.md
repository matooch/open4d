# 122 · Bake to keys and cache tags; dynamics UI

Status: done · Area: I · Depends on: 115, 093 · Observations: – · Touches: dynamics, ui

## Goal
Bake simulations to keys or a cache tag; a scene dynamics settings panel.

## Inputs
- Area design: [../../roadmap/areas/I-dynamics.md](../../roadmap/areas/I-dynamics.md)

## Design decisions (planner)
- Bake uses 093. Cache tag stores per-frame transforms in the native file.

## Done when
- [x] Baked playback equals live simulation at the recorded frames.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- **Native coverage:** rigid key bake matches all 31 sampled local poses to 1e-9,
  undo restores live dynamics, and native chunk round-trip replays rigid cache
  poses in both headless and viewport evaluation with simulation disabled.
  Cached cloner and soft-body meshes equal their pre-bake meshes.
- **Evaluation:** the viewport scheduler and headless evaluator share the dynamics
  stage. Worlds start from the animated start pose; a tracked kinematic lift pushes
  a dynamic body in both evaluators.
- **UI:** a simulation settings window creates no document node until edited;
  selection can bake to keys or a cache tag. Settings and bake commands are undoable.
- **Scope:** key bake replaces rigid position/rotation tracks. Cloner and soft
  bodies use cache tags. Cache playback overrides output but does not feed live
  collisions. No reference parity claimed.
- **Files:** DYNAMICS §5.2, simulation/cache and app scheduler/commands, animation
  sample bake helper, editor window and icons, generated command reference and tests.
- **Gate:** `GATE: 2454 tests passed, 0 failed`; `GATE: all passed`.
