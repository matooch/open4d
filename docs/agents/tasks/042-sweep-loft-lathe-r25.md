# 042 · Sweep rail/banking/scale, Loft linear/multi, Lathe partial/movement, cap fixes

Status: waiting: O20 · Area: C · Depends on: – · Observations: O20, O5, O15 · Touches: generators

## Goal
The R25 options of the spline generators observed in O20, the loft N-gon caps (O5), and the sweep caps on open paths (O15).

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O20: [../../parity/generators-3.md](../../parity/generators-3.md) and its golden cases
- O5: [../../parity/rechecks.md](../../parity/rechecks.md) and its golden cases
- O15: [../../parity/rechecks-2.md](../../parity/rechecks-2.md) and its golden cases
- docs/parity/generators-3.md, rechecks.md, rechecks-2.md
- crates/generators/src/spline_gens.rs, GENERATORS2.md §7–10
- the KNOWN_MISMATCH entries in generators_parity.rs (sweep/helix, loft/mesh-subdivision): retry them with the new data

## Design decisions (planner)
- Sweep: the rail spline scales the profile; banking on/off; end scale and rotation interpolated along the path; caps on open paths (N-gon by default; triangles case O15).
- Loft: linear interpolation; three profiles; R25 N-gon caps (16-gons on the default circles, O5).
- Lathe: partial angle with caps, and movement (helical).

## Done when
- [ ] GENERATORS2.md extended.
- [ ] O20 sweep, loft and lathe cases plus rechecks match. Old KNOWN_MISMATCHes are resolved or keep their reasons.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
The shared Caps/Bevel tab (044).

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
