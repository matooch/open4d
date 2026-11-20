# 193 · Parity tests use Extrude's Auto direction

Status: ready · Area: C · Depends on: 044, 045 · Observations: O20, O46, O48 (existing cases) · Touches: generators (tests only, unless a rule turns out wrong)

## Goal
Brief 044 gave Extrude a real Direction parameter (Auto = along the spline normal). Before that, six
parity tests worked around the missing rule: they compute the sign themselves (`auto_sign` in
`crates/generators/tests/generators3_parity.rs` and `spline_tools_parity.rs`, the `total < 0.0` sign
in `text_parity.rs`) and then set Direction = Custom with that sign. So the tests never check Auto.
Make every case that was observed with the default Direction use Auto, so the generator's own rule is
what the golden data checks.

## Inputs
- `crates/generators/tests/{generators3_parity,spline_tools_parity,text_parity}.rs`, the helpers named above
- `docs/specs/GENERATORS2.md` (Extrude Direction, from brief 044)
- `crates/generators/src/spline_gens.rs` (the Auto implementation)

## Design decisions (planner)
- Tests only: remove the `auto_sign` helpers and the hand-computed sign; leave Direction at its default
  (Auto) for every case whose `case.toml` does not set a direction. Keep Custom only where the case
  itself sets one.
- If a case then fails, do **not** restore the workaround. Compare our Auto with the sign the old
  helper computed, find which rule differs, and either fix the Auto rule in `spline_gens.rs` (spec
  first, in GENERATORS2.md) or, if you cannot after three hypotheses, put the case in that test's
  `KNOWN_MISMATCH` with the reason "Auto direction differs: <what you found>".

## Done when
- [ ] No parity test computes an extrude sign itself; `grep -rn auto_sign crates/generators/tests` is empty.
- [ ] Every case still passes, or is pinned with the reason above.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
