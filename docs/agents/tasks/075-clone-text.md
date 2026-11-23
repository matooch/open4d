# 075 · Clone-toolset Text

Status: done (parity-pending: font metrics and outlines) · Area: E · Depends on: 039, 065 · Observations: O31, O48 · Touches: clone-engine, generators

## Goal
Text as clones: letters, words and lines as clone groups, so effectors act on them.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O31: [../../parity/clone-generators-2.md](../../parity/clone-generators-2.md) and its golden cases
- O48: [../../parity/text.md](../../parity/text.md) and its golden cases
- docs/parity/text.md (O48)
- docs/parity/clone-generators-2.md
- brief 039 (text spline)

## Design decisions (planner)
- Reuse the text spline and extrude from 039; group the resulting pieces per letter/word/line as clones with matrices at their origin (rule from O31).
- Fonts: bundled open font (never the reference fonts); parity at level C on the outline, exact on clone counts and order.

## Done when
- [x] Clone counts and order match both O31 cases; existing O48 layout coverage passes.
- [ ] Numeric centres match (pending a reference capture using the bundled font).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- **O31:** `clone-text/defaults` and `clone-text/depth` are both rebuilt from
  their manifests. Counts/order, orientation, visibility, colour, weight, z pivot,
  and mesh Z bounds are exact (2/2). The two XY pivot mismatches are pinned at the
  fixture tolerance (max error 43.945312 and 9.326172). No exact font-outline or
  face-topology match is claimed.
- **O48:** the existing Text spline harness still exercises all 14 fixtures; its
  15 tests pass after sharing the new glyph-aware font layout. These validate
  layout rules without claiming a font-outline match.
- **Rules:** `depth` extrudes toward +Z (20/50); x is the advance midpoint
  (`depth`), y is the line ink midpoint (`defaults`), z is the front plane.
  Per-letter/word/line groups keep placement order; intra-group geometry and
  Plain effector displacement are tested. Caps keep counters open and retain
  closed, consistently wound rim edges.
- **Deviations:** caps use a constrained triangulation rather than copied N-gon
  topology. Only the active grouping's effects are applied; Delay requires
  history support specific to this generator. No reference fonts were accessed.
- **Files:** TEXT §7, `forge-text` glyph layout, Text spline helper, Clone Text
  generator/tests, Spade dependency and registry/icon mappings.
- **Follow-up:** capture a freely licensed shared font with explicit glyph/group
  transform samples, word/line effects, and non-default axes.
- **Gate:** `GATE: 2452 tests passed, 0 failed`; `GATE: all passed`.
