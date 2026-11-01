# 065 · Clone data, cloner node, Linear / Radial / Grid / Honeycomb modes, instance modes

Status: ready · Area: E · Depends on: 050 · Observations: O8, O11, O21 · Touches: clone-engine, plugin

## Goal
A cloner node whose Linear, Radial, Grid Array and Honeycomb modes produce the same clone matrices as the reference, plus the instance modes. All matching O8/O11/O21 clone samples pass at level B; the expanded mesh goldens match.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O8: [../../parity/cloner.md](../../parity/cloner.md) and its golden cases
- O11: [../../parity/followups-1.md](../../parity/followups-1.md) and its golden cases
- O21: [../../parity/cloner-3.md](../../parity/cloner-3.md) and its golden cases
- docs/parity/cloner.md, cloner-3.md (parameter tables, samples)
- docs/specs/MOGRAPH.md (existing spec; extend it)
- crates/clone-engine/src/lib.rs (current stub)
- crates/parity-harness/src/samples.rs (kind `clones`)
- tests/golden/cloner/

## Design decisions (planner)
- `CloneData` as in the area design (struct of arrays: matrices, colors, weights, visible, uvw, index, time). Effectors (068) mutate it later; keep it `Clone` and cheap to copy.
- `Cloner` is a generator node (`Category::Generator`) registered through `forge-plugin`; its children are the prototypes. Evaluation: evaluate children once, then produce `CloneData`, then expand to geometry for export/Make Editable in clone order (the goldens show the order).
- Modes: Linear (count, offset, mode per step / end point, step rotation/scale, cumulative), Radial (count, radius, plane, start/end angle, offset, offset variation with seed), Grid (count xyz, size per step / endpoint, forms cubic/sphere/cylinder, fill), Honeycomb (width/height count, size, orientation, offset direction). Take the exact formulas from the samples; record each in MOGRAPH.md with the case that proves it.
- Instance modes: Instance, Render Instance, Multi-Instance only change how the viewport/renderer consumes clones; geometry export is the same. Store the mode; the viewport hook is 079.
- Add `samples::clones` comparison support for any new fields you need, without loosening tolerances.

## Done when
- [ ] MOGRAPH.md updated with every rule and its case.
- [ ] All O8/O11/O21 linear/radial/grid/honeycomb samples pass, or are listed in `KNOWN_MISMATCH` with a reason.
- [ ] Cloner registered as a node; undo, serialization and attribute UI derive from its descriptors.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Object mode (066), clone order with several children (067), effectors (068+).

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
