# 092 · Speed graph mode and speed editing

Status: done (parity-pending: G5) · Area: G · Depends on: 091 · Observations: G5 · Touches: ui, anim

## Goal
A speed (velocity) view of curves, with editing that maps back to tangents.

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)
- GUI session G5 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**

## Design decisions (planner)
- Speed = analytic dv/dt (089). Editing a speed handle scales the adjacent tangent lengths (own design, documented in ANIM.md).

## Done when
- [x] Unit test: editing speed at a key changes tangents so the new speed matches within 1e-6.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- **Native result:** signed speed mode plots the analytic derivative and edits each
  spline-side speed through `key.set_speed`. Tests check exact endpoint speeds
  within 1e-6, collapsed handles, broken/weighted handles, degrees per second, and
  a real UI drag followed by a single undo.
- **Edit design:** scale vertical handle length, preserving its resolved time
  length; if collapsed, use one third of the adjacent span. Linked spline sides
  receive the same speed. Key times and values remain unchanged.
- **Files:** ANIM §1.5c; animation interpolation/tangent helpers and speed tests;
  F-curve manager and UI tests; command, icon, and generated reference.
- **Parity pending G5:** no speed-view capture is available. Observe the display
  units, one-sided grip interactions, and what the reference does with broken
  and weighted handles. Discrete/quaternion tracks are not treated as scalar speed.
- **Gate note:** the first run hit the unrelated graph test
  `a_shared_node_runs_once_per_evaluation` (2 vs 1); no graph changes were made.
- **Final gate:** `GATE: 2454 tests passed, 0 failed`; `GATE: all passed`.
