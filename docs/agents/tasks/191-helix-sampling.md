# 191 · R25 Helix Bézier construction (sampling parity)

Status: ready · Area: B · Depends on: 028, 042 · Observations: O17, O20, O45 · Touches: generators

## Goal
The R25 Helix is built as `n + 1` Bézier points with handles "a third of the piece length"
(`crates/generators/src/splines.rs`, `bezier`), a guess. It leaves `splines-2/helix-uniform` 3.5e-4
off (tolerance 1e-4), and that error is amplified in every case that uses a Helix: `sweep/helix`,
`sweep/rail`, `sweep/banking`, `sweep/scale-end`, `spline-rail/defaults`, `spline-wrap/defaults`.
Derive the real construction from the data and make `helix-uniform` match exactly.

## Inputs
- `tests/golden/splines-2/helix-uniform` (901 points on the first ring), `crates/generators/tests/spline_sampling.rs`
- Brief 028 Report: the app's handle read-back is 0.75 × the geometric offset and the default Helix handle reads 6.279 against our 6.283 (`docs/specs/SPLINES.md`)
- The Helix-based cases above (their KNOWN_MISMATCH comments name the 3.5e-4 route error)

## Design decisions (planner)
- Fit the handle direction and length rule (and, if needed, the point placement) to `helix-uniform` first; three hypotheses budget, e.g. handle length = chord/3, = (π/2·r·Δθ)·k/3 for the circular part, = the tangent-matched cubic of a circular arc (4/3·tan(Δθ/4)·r), each combined with the pitch component.
- When `helix-uniform` matches, re-run every Helix-based case and remove the ones that now match from KNOWN_MISMATCH (they stay pinned for their other, separately recorded causes otherwise; update each comment to the remaining cause).
- Update SPLINES.md / GENERATORS2.md with the derived rule and the proving case.

## Done when
- [ ] `helix-uniform` matches within 1e-4, or the brief reports the three hypotheses tried with their max errors.
- [ ] Every Helix-based case is re-checked and its KNOWN_MISMATCH entry is removed or its reason updated.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: parity N/M; hypotheses tried with errors; cases un-pinned; files changed; last gate lines.
