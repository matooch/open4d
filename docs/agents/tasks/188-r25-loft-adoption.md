# 188 · Adopt the R25 Loft ring and quad order (D18)

Status: done · Area: C · Depends on: 042 · Observations: O20 · Touches: generators, io

## Goal
Make Loft follow R25 per decision D18, so `loft/three-circles` and `loft/linear` match exactly.

## Inputs
- Brief 042 Report and `docs/specs/GENERATORS2.md` §9 and §9a (the conflict write-up)
- Cases: R25 `loft/three-circles`, `loft/linear`, `rechecks/loft-defaults`; R20 `loft/defaults`, `loft/mesh-subdivision`
- `crates/generators/tests/sweep_loft_r25.rs` (shows U+1 sampling and the R25 quad order match all 144 points)

## Design decisions (planner)
- R25 rule: U points per ring (not U−1) and quads `[r(j,i), r(j+1,i), r(j+1,i+1), r(j,i+1)]`; N-gon caps as in 042.
- Per D18, R20 cases that now differ move to KNOWN_MISMATCH with `superseded by R25 (D18): <case>`; nothing else changes for them.
- Update §9 to the R25 rule and turn §9a into a short D18 note.

## Done when
- [x] `loft/three-circles` and `loft/linear` match exactly in `generators_parity` and `export_parity`; every other currently-matching case still matches.
- [x] Superseded R20 cases are listed with the D18 reason.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: loft/three-circles and loft/linear exact in generators_parity and export_parity (loft 2/6; generators_parity 35 exact, export_parity 110/118). Superseded R20 cases (KNOWN_MISMATCH, `superseded by R25 (D18): loft/three-circles`): loft/defaults, loft/three, loft/caps-off (loft/mesh-subdivision was already a mismatch). Files: crates/generators/src/spline_gens.rs, tests generators_parity.rs, sweep_loft_r25.rs, io export_parity.rs, GENERATORS2.md, STATUS.md, BACKLOG.md, this brief. Gate: GATE: all passed.
