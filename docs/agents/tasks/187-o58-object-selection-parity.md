# 187 · O58 object-command and selection parity

Status: ready · Area: B · Depends on: 031, 034 · Observations: O58 · Touches: modeling, command, app

## Goal
Read the O58 golden cases that no test consumes yet and make them match:
`modeling-tools/arr-f-*` (array_command, duplicate, arrange, randomize, transfer — brief 031) and
`modeling-tools/sel-*` (grow, convert_selection tolerant, select_connected; expected output in `selection.json`).

## Inputs
- Observation form: [../../parity/modeling-tools-3.md](../../parity/modeling-tools-3.md) (O58)
- Specs: `docs/specs/MODELING.md` (§ object commands, selection), brief 031's parity-pending rules
- Existing tests: `crates/modeling/tests/arrange.rs`, `crates/modeling/tests/tools_parity.rs`

## Design decisions (planner)
- A parity test per family that discovers the cases by `tool` (like `tools_parity.rs`), so new cases are picked up automatically.
- Selection cases compare selections (sorted indices per component type), not meshes.
- Randomize uses the reference seed only if the case records it; if the stream can't be matched, compare statistically (Level C) and say so in the spec.

## Done when
- [ ] Every `arr-f-*` and `sel-*` case is matched, or listed in KNOWN_MISMATCH / UNSUPPORTED with a reason and a parity-pending rule plus a follow-up case in the form's Notes.
- [ ] Brief 031's parity-pending rules are confirmed or corrected in the spec.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: parity N/M.
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations, files changed, open questions, last gate lines.
