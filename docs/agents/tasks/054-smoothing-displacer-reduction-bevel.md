# 054 · Smoothing deformer, Displacer, Polygon Reduction, Bevel deformer

Status: done (parity-pending: O19) · Area: D · Depends on: 050, 125 · Observations: O19 · Touches: modifiers, modeling

## Goal
These four deformers match O19.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O19: [../../parity/deformers-3.md](../../parity/deformers-3.md) and its golden cases
- docs/parity/deformers-3.md

## Design decisions (planner)
- Smoothing: Laplacian with iterations and stiffness. Displacer: the shader height along the normal (shaders from 125). Polygon Reduction: quadric edge collapse (reuse it for the volume mesher adaptivity). Bevel deformer: wraps the 021 bevel.

## Done when
- [x] MODIFIERS.md sections.
- [x] Cases match, or are pinned.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: four deformer node types and one parity test file; parity 4/8 geometric. Matching: `smoothing-deformer/defaults` and `iterations-10` (the Laplacian now reproduces the reference area 140,956 / volume 4,339,075), `displacer/defaults` (no shader moves nothing) and `bevel-deformer/defaults` (24 points / 26 faces). Pinned: `displacer/height-noise` (no observed UVW tag; object-box mapping gives Hausdorff 23.8, volume -2.7 %), `polygon-reduction/50` and `/defaults` (counts now exact at 58/112 and 13/22; positions differ, Hausdorff 9.9 / 37.1, area -4 % / -28 %), and `bevel-deformer/offset-20` (Subdivision/rounding not derived: flat 24/26 vs 96/98).
- Smoothing hypotheses against the golden area 141k / volume 4.3M, 10 iterations: (a) edge neighbours, lambda = 1.0 -> 94,406 / 2,376,131 (too much shrink); (b) edge neighbours, lambda = 1 - stiffness = 0.7 -> 140,956 / 4,339,075 (exact, kept); (c) polygon neighbours (diagonals), lambda = 0.3 -> 209,372 / 7,785,322 and lambda = 0.7 -> 103,651 / 2,744,246 (not it). The implemented weight is `strength * (1 - stiffness)`; strength other than 100 % and the other types are parity-pending.
- Polygon Reduction now triangulates the input and targets `round((1 - strength) * triangles)`: the default sphere has 224 triangles, so 50 % keeps 112 faces / 58 points and 90 % keeps 22 faces / 13 points, both exactly as observed and both closed all-triangle (`F = 2V - 4`). The earlier stall was the quadric loop removing duplicate/non-manifold collapses; the rewritten triangle loop uses the link condition (two shared neighbours) and reaches both targets. Geometry still differs because the reference's collapse order/cost is not a plain midpoint QEM, so both cases stay pinned on positions only.
- Rules derived: Smoothing is a Jacobi Laplacian with weight `strength * (1 - stiffness)` over edge neighbours; the Displacer samples the linked shader at the point and moves along the normal/spherical/planar direction (identity without a shader); Polygon Reduction is a QEM triangle collapse to `round((1 - s) * F)` with the link condition; the Bevel deformer insets every face, puts a quad on every sharp edge and a polygon at every vertex (the Bevel tool rejects three selected edges at one vertex).
- Deviations from the design decisions: "Bevel deformer wraps the 021 bevel" only holds for point and polygon modes; edge mode uses a full-mesh chamfer because the tool refuses the all-edge selection. "Polygon Reduction reuses quadric edge collapse for the volume mesher adaptivity" was not extended to the volume mesher (it keeps its observed block merge).
- Files changed: `crates/modifiers/src/{smoothing,displacer,reduction,bevel_deformer}.rs` (new), `crates/modifiers/src/lib.rs`, `crates/modifiers/Cargo.toml`, `crates/modifiers/tests/deformers3_parity.rs` (new), `crates/ui/src/icon_table.rs`, `docs/specs/MODIFIERS.md` §22, `docs/parity/STATUS.md`, `docs/roadmap/BACKLOG.md`.
- Open questions: a UVW tag observation for the Displacer (or the rule for procedural objects); the Polygon Reduction collapse order/cost that reproduces the reference positions; the Smoothing type/keep-displacements behaviour; the Bevel Subdivision profile. These would unpin the four remaining cases.
- The last lines of the gate output: `GATE: 2514 tests passed, 0 failed` and `GATE: all passed`.
