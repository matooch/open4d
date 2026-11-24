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
- Result: four deformer node types and one parity test file. Parity 2/8 geometric: `displacer/defaults` (no shader moves nothing) and `bevel-deformer/defaults` (24 points / 26 faces) pass the volume metric. Six cases are `KNOWN_MISMATCH` with measured reasons: `smoothing-deformer/defaults` and `iterations-10` (our Laplacian shrinks less: area 238k vs 141k, volume 9.3M vs 4.3M), `displacer/height-noise` (no observed UVW tag; object-box mapping gives Hausdorff 23.8, volume -2.7 %), `polygon-reduction/defaults` and `/50` (quality stopping rule not derived: 104/108 vs 13/22 and 58/112), `bevel-deformer/offset-20` (Subdivision/rounding not derived: flat 24/26 vs 96/98).
- Rules derived: Smoothing is a Jacobi Laplacian with `lambda = strength * stiffness` over edge neighbours; the Displacer samples the linked shader at the point and moves along the normal/spherical/planar direction (identity without a shader); Polygon Reduction is a quadric edge collapse with a manifold/link check; the Bevel deformer insets every face, puts a quad on every sharp edge and a polygon at every vertex. The full-edge chamfer was needed because the shared Bevel tool rejects three selected edges at one vertex, which a cube has at every corner. **Parity-pending**: the Smoothing type/keep-displacements and its exact shrink weighting, the Displacer UVW/tag mapping, the reduction quality criterion, and the bevel rounded profile.
- Deviations from the design decisions: "Bevel deformer wraps the 021 bevel" only holds for point and polygon modes; edge mode uses a full-mesh chamfer because the tool refuses the all-edge selection. "Polygon Reduction reuses quadric edge collapse for the volume mesher adaptivity" was not extended to the volume mesher (the mesher keeps its observed block merge).
- Files changed: `crates/modifiers/src/{smoothing,displacer,reduction,bevel_deformer}.rs` (new), `crates/modifiers/src/lib.rs`, `crates/modifiers/Cargo.toml`, `crates/modifiers/tests/deformers3_parity.rs` (new), `crates/ui/src/icon_table.rs`, `docs/specs/MODIFIERS.md` §22, `docs/parity/STATUS.md`, `docs/roadmap/BACKLOG.md`.
- Open questions: a UVW tag observation for the Displacer (or the rule for procedural objects); the Polygon Reduction quality factor and stop criterion; the Smoothing type/keep-displacements behaviour; the Bevel Subdivision profile. These would unpin the six cases.
- The last lines of the gate output: `GATE: 2514 tests passed, 0 failed` and `GATE: all passed`.
