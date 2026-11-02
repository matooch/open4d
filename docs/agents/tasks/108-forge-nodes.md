# 108 · Node expression engine forge-nodes

Status: ready · Area: H · Depends on: 016 · Observations: – · Touches: nodes (new)

## Goal
A typed node graph engine for expressions (and later node materials).

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)

## Design decisions (planner)
- Typed ports, topological evaluation, cycle check, node types registered through the plugin registry; Rhai node for custom code (016).

## Done when
- [ ] Unit tests: math graph evaluates, cycle rejected, graph serializes.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
UI (109).

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
