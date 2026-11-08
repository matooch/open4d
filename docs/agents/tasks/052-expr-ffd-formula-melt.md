# 052 · Expression library; FFD, Formula and Melt deformers

Status: done · Area: D · Depends on: 050, 063 · Observations: O9, O45 · Touches: expr (new), modifiers

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
- [x] EXPR.md spec; MODIFIERS.md §16–19 (§9–11 were taken).
- [x] Cases match, or are pinned (Melt statistical).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `crates/modifiers/tests/ffd_formula_melt_parity.rs` added (7 cases). FFD 3/3 exact (`ffd/defaults`, `ffd/point-moved`, `ffd/ffd-grid-5`), Formula 2/2 exact (`formula/defaults`, `formula/frame-10`), Melt 0/2 exact: `melt/defaults` and `melt/strength` are in `KNOWN_MISMATCH` (exact comparison asserted to fail) and checked by structural invariants on golden and ours. `UNSUPPORTED` empty. All seven golden meshes are really deformed (checked against the undeformed cube: 80, 96, 124, 162, 162, 162 points differ; `ffd/defaults` is the undeformed cube, as its row says), so the scripted-export caveat of O45 does not apply to this brief.
- Rules derived (MODIFIERS.md §16-18): FFD = Bernstein (Bezier) volume of the lattice offsets, degree = points - 1 per axis, lattice x fastest, points outside the lattice box are not moved (`ffd/point-moved`: the y = +-200 points stay; `ffd/ffd-grid-5`: 80 points move, error 4e-7). Formula Y Radial: `p' = (X,Y,Z) + y * d * size_y/2`, `u = sqrt((x/hx)^2 + (z/hz)^2)`, `t` = seconds (`formula/defaults` frame 0 and `formula/frame-10`, all 324 points). Melt: each point keeps its xz direction and its distance grows by a factor `k` that is one per ray (`melt/defaults`, 24 rays), `(k-1)` at 50 % over 25 % is 1.6905 for every ray; `y <= 0` collapses to 0; on the top cap `y' = y*exp(-8 s^2 F)` with `F` independent of the strength and `F = 1` on the axis (121.306 = 200 e^-0.5, 27.067 = 200 e^-2). My first reading ("inner rays are flat") was wrong and corrected after checking every cap point.
- Parity-pending: the Melt values (the reference noise; `k(theta)`, the spread curve `g(s)` is a fit to one ratio, `F(r, y, theta)` is a fit and its y law is not derived: three hypotheses for `F`/`g` (smoothstep-like, `1-(1-s)^p`, `1-exp(-c s)`) did not give a clean closed form); the Formula effects other than Y Radial, `v`, boundary behaviour outside `size`, bad-formula handling; the FFD fade at the lattice boundary.
- Deviations: new crate `forge-expr` as the brief asks. The node-graph expression system in `forge-nodes` is a typed port graph with a Rhai script node, not a one-line formula syntax (`^`, case-insensitive functions, `PI`), so it is not reused; EXPR.md says so. MODIFIERS sections are §16-19 (§9-11 are taken). Extra functions beyond the brief: `atan2 pow round sign`. New icon `ffd.svg`; `formula.svg` and `melt.svg` reused.
- Files changed: crates/expr/{Cargo.toml, src/lib.rs} (new), crates/modifiers/{Cargo.toml, src/{lib,common,ffd,formula,melt}.rs, tests/ffd_formula_melt_parity.rs}, crates/ui/{src/icon_table.rs, icons/ffd.svg}, Cargo.lock, docs/specs/{EXPR.md (new), MODIFIERS.md}, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions / follow-up cases: see MODIFIERS.md §19 (Melt with both randomness values 0 at strengths 0/10/50/100, Melted Size 100/200, Radius 50/200, Noise Scale 50; Formula with `d = 0.1`, `d = u`, `d = v`, each other effect, a non-uniform size, `X = x*2`; FFD with a point moved on x and z, 2 points per axis, a smaller `grid_size`).
