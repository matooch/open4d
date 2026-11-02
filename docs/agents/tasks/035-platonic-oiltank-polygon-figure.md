# 035 · Platonic, Oil Tank, Polygon and Figure primitives

Status: ready · Area: C · Depends on: 033 · Observations: O18 · Touches: generators

## Goal
Four more solid primitives with R25 defaults. Platonic, Oil Tank and Polygon match the goldens. Figure is our own procedural mannequin (level C).

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O18: [../../parity/primitives-3.md](../../parity/primitives-3.md) and its golden cases
- docs/parity/primitives-3.md (parameter tables, cases, notes)
- tests/golden/{platonic,oil-tank,polygon-prim,figure}
- crates/generators/src/round.rs (pattern), platonic.rs (sphere types share code)

## Design decisions (planner)
- Platonic: 6 types with `segments` subdivision. Reuse `platonic.rs` from the sphere types where the geometry agrees. Tetra is not centred (O18 note); Bucky is slightly off-centre.
- Oil Tank: a cylinder with spherical-cap ends (cap height), plus slice. Reuse `round.rs`.
- Polygon: a flat quad or triangle with segments, orientation +Y default.
- Figure: **never copy or tabulate the golden mesh** (CLAUDE.md rule 2). Build a procedural mannequin from capsules and boxes that respects the height and segments parameters. The test checks only the bbox (±5 %) and that changing the parameters changes the mesh.

## Done when
- [ ] PRIMITIVES3.md (new spec) with params tables citing O18.
- [ ] platonic, oil-tank and polygon-prim cases match. The Figure test is level C.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Landscape (036).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added; parity N/M (`UNSUPPORTED` / `KNOWN_MISMATCH` with reasons).
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations from the design decisions, and why.
- Files changed.
- Open questions or data that would settle them (the reviewer turns these into follow-up forms).
- The last lines of the gate output.
