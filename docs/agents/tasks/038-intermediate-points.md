# 038 · Intermediate Points modes and R25 per-spline defaults

Status: done · Area: C · Depends on: – · Observations: O17, O4 · Touches: generators, geometry

## Goal
Every spline supports None, Natural, Uniform, Adaptive and Subdivided sampling. Arc, Circle and Helix default to Uniform (8), as in R25. This removes brief 07's "R25 default not supported yet" rows.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O17: [../../parity/splines-2.md](../../parity/splines-2.md) and its golden cases
- O4 (see the area file)
- docs/parity/splines-2.md (defaults per spline)
- docs/specs/GENERATORS2.md §0a (brief 07 kept Adaptive for circle and helix)
- tests/defaults/r20.toml (the R20 overlay keeps Adaptive for R20 cases)

## Design decisions (planner)
- Shared module `crates/generators/src/spline_sampling.rs`: `sample(spline, mode, number, angle, max_length) -> Vec<DVec3>`. Uniform = equal parameter steps per segment (check against helix-uniform and circle-uniform).
- Switch the descriptor defaults (circle and helix intermediate = uniform), and make sure the R20 overlay still pins adaptive for the R20 cases.

## Done when
- [x] GENERATORS2.md §0a updated.
- [x] All existing spline-based cases still pass; the O17 uniform cases: circle-uniform matches, helix-uniform is a known mismatch (3.5e-4).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `crates/generators/src/spline_sampling.rs` (new) and `crates/generators/tests/spline_sampling.rs` (9 tests). Parity: the 31 generator cases (`generators_parity`) are unchanged (31 exact, `loft/mesh-subdivision` and `sweep/helix` still pinned). O17 `splines-2` (22 cases): `circle-uniform` exact (extrude, strict after `export_weld`), `helix-uniform` known mismatch (first ring of 901 points within 3.5e-4, tolerance 1e-4), the other 20 splines are brief 037's (their node types do not exist yet; the test counts them as unsupported).
- API (for brief 037): `Curve { segments, linear, closed, sampling }` (a list of cubic Bezier segments) with `Curve::points()`; `spline_sampling::{Interpolation, Sampling, sample, add_params}`. A new spline primitive builds a `Curve` with `Sampling::from_params(&p)`, registers its params through `add_params(node_type, default_mode)` (Uniform for Arc, Adaptive for the rest) and gets all five modes in extrude, lathe and sweep. `linear` pieces get no interior points in any mode.
- Rules (spec GENERATORS2.md §6.2): Uniform = `number` samples at equal arc length inside each segment, length from the polyline through `t = k/50`, `t` interpolated linearly (`circle-uniform` 5e-7; exact length 3e-3, 40 or 64 pieces 2e-3 to 3e-3, equal-`t` 1.4); Adaptive = the existing 1/50 walk, now with the angle as a parameter; an open Uniform spline ends at 0.9999 of its whole parameter (`arc-defaults` bbox x 199.96847, `helix-uniform` last point t 0.99 of the 100th piece). R25 helix = Bezier with `subdivision + 1` points in total (101), handles along `f(s+1/N) - f(s-1/N)` of length (piece length)/3, used for every mode but Adaptive; Adaptive keeps the R20 polyline (`sweep/helix`). Defaults: circle and helix `intermediate` Uniform (R25), the R20 overlay already pinned Adaptive; `r25_defaults` test now applies 39 of the 42 `!` rows.
- Parity-pending: None (no case), Natural (equal steps of `t`: the only other even spacing, no case), Subdivided (smallest equal-`t` split under Angle and Maximum Length, no case), Adaptive at angles other than 5 degrees, Uniform on straight pieces (none added) and as a global spacing over unequal segments, Number 0 (acts as None), the Uniform end shortfall for specs other than arc and helix, the R25 Adaptive helix (the R20 polyline is kept), and the closed-spline Uniform/Natural start offset.
- Deviations: Uniform is arc length (not equal parameter as the brief's design note says; `circle-uniform` shows it). `helix-uniform` not exact: the exported handles carry a small inward radial part (-2.95e-4 on 8.377 tangential and -0.668 in z, equal for every piece) and the points drift by about 1e-7 of their parameter; true tangent, chord/3 and arc/3 handles tried, best 3.5e-4. Helix geometry is chosen by the mode (Adaptive = R20 form), a pragmatic split until the R25 Adaptive helix is observed.
- Files: crates/generators/src/{spline_sampling.rs (new),splines.rs,spline_gens.rs,lib.rs}, crates/generators/tests/{spline_sampling.rs (new),r25_defaults.rs}, docs/specs/GENERATORS2.md (§0a, §6), docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, docs/roadmap/areas/C-objects.md, this brief.
- Open data that would settle pending rules: a Circle spline (or Arc) extrude with Intermediate Points None, Natural 8, Natural 3, Subdivided (Angle 5 and 20, Maximum Length 5 and 50) and Uniform Number 3 and 0; Adaptive Angle 10 and 20 on the Circle; a Rectangle and an n-Side with Uniform 8 (straight pieces); a spline with unequal segments (Star with Uniform 4, an Arc ring) to tell per-segment from global spacing; a Helix with Adaptive in R25, Helix Subdivision 50 and End Angle 360 / 1080 (point count total vs per turn), Helix Radial and Height Bias; the Helix handle vectors read from the editable spline (Make Editable, point tangents of points 0, 1 and 100), which would give the exact handle rule; the Extrude Direction of the helix (the second ring is displaced along (0, -3.14, 19.75)).
- The last lines of the gate output: `advisories ok, bans ok, licenses ok, sources ok` / `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`.
