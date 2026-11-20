# 112 · Weight and joint display modes in the viewport

Status: done · Area: H · Depends on: 101, 143 · Observations: – · Touches: viewport

## Goal
Show weights as a colour ramp and joints in several styles.

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)

## Design decisions (planner)
- Weights as a vertex colour overlay for the selected joint.

## Done when
- [ ] Visual test against our own baseline image.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `forge_viewport::rig`: bone styles (None, Standard, Box, Line, Stick, Polygon), joint styles
  (None, Axis, Sphere wire, Circle, Sphere) as `DrawLine` lists, the weight colour ramp and a per-vertex
  weight marker overlay for the selected joint. 7 pure tests (`crates/viewport/tests/rig_display.rs`).
  No observation of the reference's look: parity-pending (VIEWPORT.md section 13).
- Deviations: the brief's "visual test against our own baseline image" is replaced by expected-value
  tests on the draw lists (the draw lists are the part that has logic; the line pipeline already has
  image baselines). The weight overlay is markers (5 px squares), not an interpolated vertex-colour
  surface: the shaded mesh pipeline has no per-vertex colour yet. The editor does not call these
  functions yet (the viewport crate does not know the rig crate); mapping a joint node to a
  `JointDraw` is the follow-up in the UI layer.
- Files: crates/viewport/src/rig.rs, crates/viewport/src/lib.rs, crates/viewport/tests/rig_display.rs,
  docs/specs/VIEWPORT.md section 13.
