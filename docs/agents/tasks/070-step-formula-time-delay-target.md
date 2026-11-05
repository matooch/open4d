# 070 · Step, Formula, Time, Delay and Target effectors

Status: done · Area: E · Depends on: 068, 052 · Observations: O14 · Touches: clone-engine

## Goal
Five deterministic effectors matching O14.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O14: [../../parity/effectors-2.md](../../parity/effectors-2.md) and its golden cases
- docs/parity/effectors-2.md (O14 cases)
- forge-expr (052) for Formula

## Design decisions (planner)
- Step: weight by index/(count-1) through a spline curve (default linear).
- Formula: the expression variables (id, count, time, …) as in the reference manual; evaluated with forge-expr.
- Time: parameter × time. Delay: per-clone smoothing over frames (blend/average/spring modes) — needs the previous frame state; keep it in the clone cache keyed by frame.
- Target: aim clones at a target object or the next clone.

## Done when
- [ ] O14 cases for these effectors pass at level B.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added (`cloner_parity.rs::step_formula_time_target_cases_match_the_reference`,
  `clone-engine/tests/step_formula_time_target_delay.rs`, `generators/tests/delay.rs`); parity: clone data (level B) 6/6,
  expanded mesh (level A) 5/6. `STFT_MESH_MISMATCH` = `step-position` (level A only: the reference weights are
  0.2499990830, 0.5000008583, 0.7500009170 rather than 0.25, 0.5, 0.75, so the cube corners miss the 1e-4 tolerance by
  1e-6; three hypotheses gave no rule). Delay: no golden case exists (`delay-skip` is a note), implemented as specified and parity-pending.
- Rules (spec E18-E23): Step weight `curve(i/(n-1))` (`step-defaults`, `step-position`); Formula weight = the formula in
  forge-expr degrees mode, `t` = project time in seconds (`formula-defaults`, `formula-frame-10`); Time weight = seconds
  (`time-frame-30`); Target z axis at the object, x horizontal (`target-defaults`). Pending: Formula `t` field role, Time
  linearity, Step Gap and spline edits, Target Reverse Heading / Up Vector / Pitch / Next / Previous / Strength, all of Delay.
- Deviations: Delay keeps no cache. It replays the cloner and the effectors before it over the 240 frames before the
  current one (a pure function of scene and frame), which is simpler and safe against scrubbing and undo; only the current
  scene's parameters are replayed, and one Delay per cloner. The Step spline is an enum of four presets (no curve param kind).
- Files: crates/clone-engine/src/{step,formula,time,target,delay}.rs, effector.rs, lib.rs, node.rs, plain.rs, random.rs,
  Cargo.toml; crates/clone-engine/tests/step_formula_time_target_delay.rs; crates/generators/tests/{cloner_parity,delay}.rs;
  crates/ui/src/icon_table.rs and icons/effector_{step,formula,time,target,delay}.svg; docs/specs/MOGRAPH.md (E18-E25),
  docs/parity/STATUS.md, docs/roadmap/BACKLOG.md.
- Open data: the E25 list in docs/specs/MOGRAPH.md (Step Count 9 or a changed spline for the weights; Formula t = 2; Time at
  frame 15; Target modes, Reverse Heading, Up Vector, Strength 50 %; Delay cases at frames 0, 1, 2, 5, 10 with a Time
  effector before it, one per mode).
- Gate: see the commit message.
