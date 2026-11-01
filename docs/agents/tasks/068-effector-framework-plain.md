# 068 · Effector framework + Plain effector (modes, spaces, colour, weight, min/max)

Status: ready · Area: E · Depends on: 065, 057 · Observations: O8, O14, O22 · Touches: clone-engine

## Goal
The effector trait, the effector list on a cloner (in list order) and the Plain effector with all common parameters, matching O8/O14/O22 samples.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O8: [../../parity/cloner.md](../../parity/cloner.md) and its golden cases
- O14: [../../parity/effectors-2.md](../../parity/effectors-2.md) and its golden cases
- O22: [../../parity/effectors-3.md](../../parity/effectors-3.md) and its golden cases
- docs/parity/cloner.md, effectors-2.md, effectors-3.md
- brief 057 (deformer/effector common falloff) for the strength path

## Design decisions (planner)
- `trait Effector { fn apply(&self, data: &mut CloneData, falloff: &FieldList, ctx: &EvalCtx); }`; effectors are `Category::Effector` nodes linked from the cloner list. Apply in list order (O14 two-effectors cases).
- Common parameters: strength; transform mode relative/absolute/remap; space node/effector/object; P/S/R with uniform/absolute scale; colour mode (off/on/field); weight transform; U/V; visibility; min/max remap.
- Rotation composition order and space conversion must come from the samples; write each rule down with its case.

## Done when
- [ ] Plain effector cases from O8/O14/O22 pass at level B.
- [ ] MOGRAPH.md §Effectors documents the common parameter rules.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Field falloff beyond the effector default (072).

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
