# 187 · O58 object-command and selection parity

Status: done · Area: B · Depends on: 031, 034 · Observations: O58 · Touches: modeling, command, app

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
- [x] Every `arr-f-*` and `sel-*` case is matched, or listed in KNOWN_MISMATCH / UNSUPPORTED with a reason and a parity-pending rule plus a follow-up case in the form's Notes.
- [x] Brief 031's parity-pending rules are confirmed or corrected in the spec.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: parity 12/12 (selection 6/6 exact at level A; objects 6/6: array-3, array-mode, duplicate, arrange, transfer exact on the world-space file, randomize at level C).
- Rules derived, with the case that proves each:
  - Array: instances are a grid `count_x/y/z` including the original; `offset` is the total span, step `offset/(count-1)` per axis (array-3: x = 0, 150, 300); an axis with count 1 ignores its offset (array-mode); only selected polygons are copied, copy points in ascending original index (array-mode). Replaces brief 031's `copies` design for the Array command only (Duplicate keeps it, confirmed by duplicate).
  - Polygon neighbours share a point (sel-grow-poly 3x3 patch); point/edge grow as before (sel-grow-point, sel-grow-edge); tolerant convert (sel-convert-tolerant-p2poly, -e2poly) already matched.
  - Select Connected spreads over edges used by exactly two polygons (sel-connected-point-only; O26 select-connected still holds). The old point-based rule selected both cubes.
  - Brief 031 rules confirmed: Duplicate copy k at k*offset and names Cube.0.. (duplicate), Arrange line (arrange), Transfer source to target in world space (transfer), Randomize amplitudes and seed repeat (randomize).
  - Parity-pending: step `offset/(count-1)` vs `offset/2` (follow-up `arr-f-array-4`: count 4, offset 300 along x), instance order and rotation/scale rule for several axes (follow-up: 2x2x1 array with rotation), Array with no polygon selected, Select Connected in points and edges mode and for a 3-polygon edge (follow-ups: cubes sharing one corner point; T-junction), Arrange circle and spline (no-ops in the reference build; follow-up: GUI), Randomize distribution (follow-up: 30 cubes, seed 1), Transfer: the text says source P 20 but the file shows a Z-axis turn (follow-up: distinguishable H, P, B), Use Edge on.
- Deviations: the observation form was not edited (rule), so the follow-up cases are listed here and in the specs. The reference lists new objects newest first; the test writes ours reversed for arrange and transfer, and writes without welding (array-mode keeps coincident points).
- Files changed: crates/geometry/src/{selection,selection_ops}.rs and tests, crates/modeling/src/{arrange,desc}.rs and tests/arrange.rs, crates/command/src/builtin/tools.rs (sample), crates/app/tests/{o58_object_parity,o58_selection_parity,arrange_commands}.rs, crates/ui/tests/arrange.rs, docs/specs/{MODELING,EDITABLE}.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, docs/manual/reference/commands.md.
- Last gate lines: GATE: 1793 tests passed, 0 failed; GATE: all passed
