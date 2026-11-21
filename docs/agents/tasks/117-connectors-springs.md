# 117 · Connectors and springs

Status: done · Area: I · Depends on: 115 · Observations: O41 · Touches: dynamics

## Goal
Hinge, ball, slider, fixed connectors and springs.

## Inputs
- Area design: [../../roadmap/areas/I-dynamics.md](../../roadmap/areas/I-dynamics.md)
- O41: [../../parity/soft-bodies.md](../../parity/soft-bodies.md) and its golden cases
- docs/parity/soft-bodies.md (connector cases)

## Design decisions (planner)
- rapier joints; spring as a force pair.

## Done when
- [x] O41 connector cases meet thresholds.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: connector object (hinge, ball-and-socket, slider, fixed, as rapier joints) and spring
  object (force pair). Tests: `crates/dynamics/tests/joints_parity.rs` (3 golden cases),
  `joints_unit.rs` (8), `crates/app/tests/dynamics_eval.rs` (connector in the evaluation). All three
  O41 connector cases match, none pinned:
  - `hinge`: x, y per frame within 7 cm (worst 6.6 cm of a 600 cm swing, a 0.04 frame timing
    error), Bank mod 360 within 3°, distance to the pivot 301 ± 1.5 cm.
  - `ball-socket`: the same assertions; ours also equals ours-hinge within 1 cm (the reference's
    track is bit-identical to its hinge track: the type does not reach its solver).
  - `spring`: y per frame within 3 cm (worst 2.6 cm of 60), settle height within 0.5 cm of 190.037,
    x/z/rotations stay 0, first peak at frame 10 ±1.
- Rules derived: the joint anchor is the connector's own position (not the A object's centre: the
  right cube stays 301 cm from the origin); hinge axis the connector's Z, slider axis X; spring
  `k = stiffness · 100 s⁻² · m_eff`, `c = 2 · damping · sqrt(k m_eff)` (damping is a fraction of
  critical). Parity-pending: the axis choice, the 100 s⁻² (one stiffness value observed), the other
  seven connector types.
- Deviations: the spring is its own object `forge.spring` (as observed) rather than a connector
  type; the area design's "spring as a force pair" is followed.
- Files changed: crates/dynamics/src/{joints,sim,nodes,cache,lib}.rs, crates/dynamics/tests/{joints_parity,joints_unit,common}, crates/ui/src/icon_table.rs + icons/spring.svg,
  crates/app/tests/dynamics_eval.rs, docs/specs/DYNAMICS.md §3 and §7, BACKLOG, STATUS.
- Open questions: stiffness 0.5 / 2.0 and damping 0 / 50 % spring drops (the stiffness scale is a
  one-point fit); a hinge between objects whose axes are rotated (which axis the reference uses); a
  connector at a non-origin position.
- The last lines of the gate output: see the final report.
