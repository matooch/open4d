# 030 · Make Editable follow-ups from O3

Status: ready · Area: B · Depends on: 003, 028 · Observations: O3 · Touches: generators, ui

## Goal
Make Editable and Current State to Object behave like R25 in the O3 points where brief 03 guessed differently.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O3: [../../parity/make-editable.md](../../parity/make-editable.md) and its golden cases
- docs/parity/make-editable.md (cases and behaviour notes)
- docs/specs/EDITABLE.md §1 (brief 03 parity-pending rules)

## Design decisions (planner)
- Extrude (and other cap-making generators): Make Editable gives the side walls as the object, with `Cap 1` and `Cap 2` as polygon-object children carrying polygon selection tags C1/C2 (O3 extrude).
- Current State to Object keeps the **original** selected (O3 behaviour notes); the new object is named like the source, with no suffix.
- A UVW tag is added on conversion (once J/128 gives meshes UVs; until then, record it as parity-pending).
- Spline primitives → editable spline (028).
- Generators with consumed inputs: one object named after the generator (O3 subdivision).

## Done when
- [ ] EDITABLE.md §1 updated, with the parity-pending markers removed where O3 proves the rule.
- [ ] Tests for each O3 case, including the meshes in tests/golden/make-editable/.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

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
