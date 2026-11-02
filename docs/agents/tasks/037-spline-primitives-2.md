# 037 · Spline primitives II

Status: done · Area: C · Depends on: 038 · Observations: O17 · Touches: generators

## Goal
Arc, n-Side, Star, Flower, 4-Side, Cogwheel, Cycloid, Profile, Cissoid and Formula splines, plus the Circle ellipse/ring and Rectangle rounding options, matching the O17 extrude cases.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O17: [../../parity/splines-2.md](../../parity/splines-2.md) and its golden cases
- docs/parity/splines-2.md (tables, 22 cases, notes on Cogwheel coupling and Extrude direction)
- crates/generators/src/splines.rs, spline_gens.rs (extrude)

## Design decisions (planner)
- One function per spline in `splines.rs`, each returning a `Spline` (points, tangents, closed). Intermediate points come from 038.
- Formula spline uses `forge-expr` (052). If 052 is not done yet, return Unsupported for formula-defaults and pin it.
- Cogwheel: involute teeth from the standard gear formulas (pitch radius, module, pressure angle). The coupled limits are TODO in O17; clamp to the observed defaults only.
- O17 note: Extrude Direction Auto sends some splines to −z. The test harness builds the extrude the way the case says; any direction rule found goes to 044.

## Done when
- [x] GENERATORS2.md §splines extended.
- [x] All 22 O17 cases match, or are pinned.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `spline_prims.rs` (new: arc, nside, star, flower, fourside, cogwheel, cycloid, profile, cissoid, formula), circle ellipse/ring, rectangle rounding; tests `spline_sampling.rs` (O17) and `spline_prims.rs` (9). Parity: 19/22 exact (arc-defaults/angles/sector, circle-ellipse/ring/uniform, cissoid, cycloid-defaults/epi, flower, fourside x2, nside x3, profile, rectangle-rounding, star x2). Known mismatch: `helix-uniform` (as in 038). Unsupported: `cog-defaults` (tooth outline not derived, facts in spec §6.3), `formula-defaults` (no forge-expr yet, brief 052).
- Defaults (R25, registered): arc 200/100/0/90 Uniform; nside 200, 6 sides, rounding off/50; star 100/200/0 %/8; flower 100/200/8; fourside diamond 200/100/30; cycloid 25/50/75, 0..360; profile H 200/100/20/20; cissoid 400/tension 2; formula texts as observed, 20 samples; cogwheel full table; all but arc default to Adaptive 8/5/5.
- Rules and proofs: arc handle `83/150·r·turn/90°` (nside-rounding, arc-angles, rectangle-rounding); Uniform is global arc length with the open-end shortfall in length (arc-sector, arc-defaults); straight pieces of mixed curves have zero handles (arc-sector chord, 7e-7); Adaptive = tangent difference to the last sample, end piece replaced by the midpoint when the remaining turn is smaller than the closed piece (circle, cycloid, cycloid-epi, flower); automatic tangents `unit(next-prev)·min(chord)/3`, ends chord/3 (cycloid x2, flower); kite, star twist, H profile, cissoid (`x = t y`) closed forms; ring = second loop with reversed wall winding (circle-ring); Direction Auto = sign of signed area (cycloid only negative; for 044).
- Parity-pending: flower beyond defaults (ro-ri=ri and ro=2ri ambiguous), cissoid tension, cycloid 62 pieces per turn, rounding clamps, Arc "sector" label vs chord shape, Natural/Subdivided/None, Uniform on straight pieces.
- Deviations: Uniform is global rather than per segment (038 had assumed per segment; arc-sector shows global); Adaptive rule of 038 replaced (same results on the circle); Cogwheel/Formula Unsupported instead of built; test harness negates the extrude offset by the signed-area guess.
- Files: crates/generators/src/{spline_prims.rs (new), splines.rs, spline_sampling.rs, spline_gens.rs, lib.rs}, tests/{spline_prims.rs (new), spline_sampling.rs, common/mod.rs}, docs/specs/GENERATORS2.md, docs/parity/STATUS.md, docs/roadmap/{BACKLOG.md, areas/C-objects.md}, this brief.
- Open data: editable-spline point list with tangents of one Cogwheel tooth (+ ratios varied); Flower with other radii/petals; Cissoid tension 3; Cycloid with 180 degrees and other radii; Arc types segment/ring (re-observe labels); 4-Side parallelogram/trapezium; Profile L/T/U/Z; Lemniscate/Strophoid; Hypocycloid; Formula with a different sample count (needs 052); Extrude Direction with a clockwise closed spline and XZ plane; Uniform 8 on a rectangle.
- Gate (last lines): `advisories ok, bans ok, licenses ok, sources ok` / `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`.
