# 124 · Node material graph + node editor UI

Status: done (parity-pending: G7) · Area: J · Depends on: 123, 109 · Observations: G7 · Touches: material, ui

## Goal
Node-based materials using forge-nodes and the shared canvas.

## Inputs
- Area design: [../../roadmap/areas/J-materials.md](../../roadmap/areas/J-materials.md)
- GUI session G7 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G7 §3

## Design decisions (planner)
- Node material compiles to the same IR.

## Done when
- [x] A node material and an equivalent Standard material compile to equal IR.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Hosted node materials use the shared typed graph and canvas with a dedicated pure
  material registry. Surface values lower through the Standard compiler; the test
  compares exact IR with linked colour, roughness, texture, metal, normal and emission.
- Validated graph replacement is undoable and survives save/open and assignment.
  The egui test adds a Scalar from the canvas menu, wires it to roughness and undoes
  the connection. Viewport, thumbnails and still/IPR share hosted compilation.
- Expression `graph.*` command handlers moved to `forge-app` (same contract) so
  material and I/O can depend on the forge-nodes graph kernel without a dependency cycle.
- G7 §3 layout and interactions remain unobserved. The native surface subset
  uses the existing binary metal flag and does not claim reference UI parity.
- Gate: `GATE: 2492 tests passed, 0 failed`; `GATE: all passed`.
