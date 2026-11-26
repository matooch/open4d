# 220 · Bulk clone API, Luau effector, budgets

Status: ready · Area: P · Depends on: 216 · Observations: – (own design, D21) · Touches: behaviour, luau, clone-engine, fields

## Goal
Behaviours that drive thousands of clones per frame within budget: buffer-based bulk reads and
writes, and a stateful Luau effector. BEHAVIOURS.md §10, §11.

Spec: [../../specs/BEHAVIOURS.md](../../specs/BEHAVIOURS.md) (own design, D21; no parity data). Where the spec says *(open)*, decide, write the rule into the spec, and say so in the Report.

## Steps
1. `ctx:clones(cloner)` views and `write_*` (§11). Decide where the stage sits relative to fields *(open)*.
2. `forge.luau_effector` with field falloff applied to its output, cached like a behaviour.
3. `cargo bench -p forge-behaviour --bench s11` measures the §10 rows. Record numbers from lavapipe/CI and from the M5 Pro.

## Done when
- [ ] The bulk-write budget (≤ 2 ms for 1,000 clones, M5 Pro release) is met, or the gap is documented with a profile.
- [ ] The Luau effector matches an equivalent built-in effector setup exactly (for example, a plain offset with a field).
- [ ] Clone determinism and scrubbing tests pass.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
