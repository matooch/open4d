# 046 · Null display, Instance render modes, Connect weld options

Status: blocked: O20 has no null/instance/connect cases · Area: C · Depends on: – · Observations: O20 · Touches: generators, viewport

## Goal
The smaller R25 object options (Null display shapes, Instance modes, Connect weld tolerance) exist and match where observed.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O20: [../../parity/generators-3.md](../../parity/generators-3.md) and its golden cases
- docs/parity/generators-3.md, instance.md, connect.md

## Design decisions (planner)
- Null: display shape, radius, aspect, orientation (viewport only).
- Instance: instance, render instance and multi-instance modes are equal for geometry.
- Connect: weld on/off, tolerance, Phong mode.

## Done when
- [ ] GENERATORS2.md updated; tests.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: blocked, nothing implemented. docs/parity/generators-3.md (O20, 59 cases) lists only metaball, atom-array, spline-mask, boole, subdivision, symmetry, sweep, loft, lathe, platonic, oil-tank, polygon-prim, figure, landscape cases; its Notes never mention Null display shapes, Instance modes or Connect weld/tolerance/Phong. tests/golden has no null folder; connect has only the R20 cases defaults/weld-off/apart/order (weld on/off already covered); instance has only defaults (Instance mode). No R25 observation exists for Null display, Render/Multi-Instance, Connect tolerance or Phong mode, so a spec could not cite a source (CLAUDE.md rule 4).
- Follow-up: new observation form with cases null/shape-*, null/radius-aspect, instance/render-instance, instance/multi-instance, connect/tolerance-*, connect/phong-*.
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations from the design decisions, and why.
- Files changed.
- Open questions or data that would settle them (the reviewer turns these into follow-up forms).
- The last lines of the gate output.
