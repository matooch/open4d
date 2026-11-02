# 052 · Expression library; FFD, Formula and Melt deformers

Status: waiting: O45 · Area: D · Depends on: 050, 063 · Observations: O9, O45 · Touches: expr (new), modifiers

## Goal
A shared expression language for formula features, plus the FFD, Formula and Melt deformers.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O9: [../../parity/deformers-2.md](../../parity/deformers-2.md) and its golden cases
- O45: [../../parity/deformers-4.md](../../parity/deformers-4.md) and its golden cases
- docs/parity/deformers-2.md (Formula texts, FFD grid order, Melt)
- docs/parity/deformers-4.md (ffd-grid-5)

## Design decisions (planner)
- New crate `crates/expr` (`forge-expr`): a parser for `+ - * / ^`, parentheses and comparisons, with functions sin, cos, tan, asin, acos, atan, sqrt, abs, exp, log, min, max, floor, ceil, clamp, mod, and constants PI. Function names are case-insensitive (O9: `Sin(...)*2.0*PI`). Variables are bound by the caller.
- FFD: a trilinear (2 points per axis) or Bernstein grid deformation. The grid points are stored as a `Vec3List` param, in x-fastest order (O9 note).
- Formula: effect modes (Manual, Spherical, Cylindrical, X/Y/Z Radial) with d(u,v,x,y,z,t).
- Melt: noise-driven. Compare statistically unless the noise matches (063).

## Done when
- [ ] EXPR.md spec; MODIFIERS.md §9–11.
- [ ] Cases match, or are pinned (Melt statistical).
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
