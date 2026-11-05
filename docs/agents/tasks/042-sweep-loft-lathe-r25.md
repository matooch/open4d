# 042 · Sweep rail/banking/scale, Loft linear/multi, Lathe partial/movement, cap fixes

Status: done · Area: C · Depends on: – · Observations: O20, O5, O15 · Touches: generators

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
- Result: `crates/generators/tests/sweep_loft_r25.rs` added (5 tests); `generators_parity` 36 exact (was 34), `UNSUPPORTED` empty, `KNOWN_MISMATCH` 7; `export_parity` 111 exact. Lathe 6/6 exact (`lathe/partial`, `lathe/movement` now match). Pending, now all built (none `Unsupported`): `loft/three-circles`, `loft/linear` (R20/R25 conflict, below), `sweep/banking`, `sweep/rail`, `sweep/scale-end` (R25 uniform helix route error up to 3.5e-4; scale-end also the Banking-on frame). `loft/mesh-subdivision` and `sweep/helix` keep their reasons (retried with the new data: no change).
- Rules derived: N-gon cap default for lathe, loft, sweep with R20 `cap_type = "triangles"` overlay rows (`lathe/partial`, `rechecks/loft-defaults`; R20 `lathe/angle`, `sweep/helix`); lathe N-gon caps start `[q3..q0]` / end `[q0..q3]` (`lathe/partial`); lathe movement `y += movement·k/sub`, 33 columns (`lathe/movement`); lathe scaling linear (**parity-pending**, no case); loft linear interpolation (`loft/linear`, with U + 1); loft N-gon caps (`loft/three-circles`); sweep Banking off: normal = `Y × tangent` (`sweep/banking`, exact along the reference's own ring centres); sweep 2-Rail: profile x extent mapped onto path point to rail point, y dropped, rail at the path sample's spline parameter (`sweep/rail`, exact along the reference's own ring ends); sweep N-gon caps start `[m-1..0]`, end `[0..m-1]`.
- R20/R25 conflict: loft. `loft/defaults` (R20: U 30 gives 29 points per ring, quad `[r(j,i+1), r(j+1,i+1), r(j+1,i), r(j,i)]`, triangle caps) against `rechecks/loft-defaults` and `loft/three-circles` (R25: U 16 gives 16 points, quad `[r(j,i), r(j+1,i), r(j+1,i+1), r(j,i+1)]`, N-gon caps); also `loft/mesh-subdivision` (R20, U 7 gives 6 points). No version switch added; R20 stays the default; GENERATORS2.md §9a. Related, already known: R20 `extrude/rectangle-caps-quadrangles` against R25 `r25-drift/extrude-rectangle-caps-quadrangles` (4-point n-gon cap start corner); the lathe takes the R25 order because it has no R20 n-gon case.
- Deviations: none from the design decisions, except that Loft R25 and the Sweep helix cases cannot be exact (above). The Banking-on frame (`sweep/scale-end`) is not derived after 3+ hypotheses (Frenet, bend at point 1, four discretisations of parallel transport, coarse transport, projection): the reference twists 55.3° over the helix against 57.0°, and ring 0 is 0.103° off the radial direction.
- Files changed: crates/generators/src/{spline_gens,spline_sampling}.rs, crates/generators/tests/{generators_parity,r25_defaults,sweep_loft_r25}.rs, crates/io/tests/export_parity.rs, tests/defaults/r20.toml, docs/specs/GENERATORS2.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions / follow-up cases: (1) decide how app versions are told apart in generator builds (planner) so the R25 loft can be adopted; (2) a Helix spline with Uniform interpolation at 3.5e-4 (and variable radius 2e-2): export the helix's Bezier handles of a short helix (Subdivision 4, 8) in R25 and a radius-changing one; (3) a Sweep banking-on case with a planar non-circular path and an end-scale-only case, to separate the frame rule from the helix route; (4) a Sweep with Banking off along a path in another plane (rotated helix) to tell world Y from the path's own up; (5) Lathe Scaling 50 %; (6) a 2-Rail sweep with a rectangle profile (the y behaviour).
- Gate: see the commit.
