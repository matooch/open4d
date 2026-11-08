# 055 · Spline Wrap, Spline and Spline Rail deformers

Status: done · Area: D · Depends on: 050, 028 · Observations: O19, O45 · Touches: modifiers

## Goal
The spline-driven deformers match O19 and O45.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O19: [../../parity/deformers-3.md](../../parity/deformers-3.md) and its golden cases
- O45: [../../parity/deformers-4.md](../../parity/deformers-4.md) and its golden cases
- docs/parity/deformers-3.md, deformers-4.md

## Design decisions (planner)
- Spline Wrap: maps the object's axis along the spline, with rotation from the spline's parallel transport frame (the same as Sweep).

## Done when
- [ ] MODIFIERS.md sections.
- [ ] Cases match, or are pinned.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `crates/modifiers/tests/spline_parity.rs`; parity 0/4 exact, 4 `KNOWN_MISMATCH` (Spline Wrap 0/2: arc 2e-3 from sampler digits x99 extension, helix frame roll; Spline 0/1: pairing/weights; Spline Rail 0/1: 1.9e-4 from the sampled Helix). `tests/golden/{wrap,wrap-f}` belong to plain Wrap, not this brief.
- Rules derived: sampled polyline model (`spline-wrap/*` end chords); Wrap mapping and `t = z` degenerate box (`spline-wrap/arc`); Rail formula and index fraction (`spline-rail/defaults`); Spline moved set (`spline-deformer/defaults`). Parity-pending: see MODIFIERS.md §20-21.
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations: the frame's initial normal is `Z x T` not Sweep's bend rule (the helix says neither matches); +X reads the object's z axis (observed).
- Files changed: crates/modifiers/src/{spline.rs,lib.rs}, crates/modifiers/tests/spline_parity.rs, crates/generators/src/{splines.rs,lib.rs} (spline polyline helper), crates/ui icons + icon_table.rs, docs/specs/MODIFIERS.md, docs/parity/STATUS.md, BACKLOG.md.
- Open questions: see the follow-up case list in MODIFIERS.md §21 (interactive exports).
- The last lines of the gate output.
