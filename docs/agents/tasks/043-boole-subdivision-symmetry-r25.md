# 043 · Boole R25 modes and numbering, Subdivision types, Symmetry planes

Status: done (Boole R25 strict numbering parity-pending) · Area: C · Depends on: – · Observations: O20 · Touches: generators, geometry

## Goal
Every Boole mode with R25 defaults (Hide new edges on: N-gon output), every Subdivision Surface type, and every Symmetry plane match O20.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O20: [../../parity/generators-3.md](../../parity/generators-3.md) and its golden cases
- docs/parity/generators-3.md (boole/r25-*, subdivision/r25-type-*, symmetry/r25-plane-*)
- docs/specs/BOOLE.md, SUBDIVISION.md

## Design decisions (planner)
- Boole with Hide new edges merges coplanar faces into N-gons (26 points with 6 duplicates in r25-drift; check the R25 cases). Keep geometric comparison where the numbering isn't derivable.
- Subdivision types: add any type O20 shows that we lack. Loop was `Unsupported`: implement Loop subdivision if a case exists.

## Done when
- [ ] BOOLE.md, SUBDIVISION.md and GENERATORS2.md updated.
- [ ] O20 cases match, or are pinned.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: **Subdivision 16/16 exact** (`r25-type-osd_loop` and R20 `scheme-osd_loop` now match; both removed from `UNSUPPORTED`). **Symmetry** `r25-plane-xy/-zy/-xz` still exact (no change). **Boole**: the four R25 cases stay in `KNOWN_MISMATCH`; the test now also asserts, per case, the R25 two-object layout (`boole_objects`: 20 / 26 / 14 / 13 points as the export), polygon sizes and `compare_geometric`. R20 Boole cases unchanged (5 geometric).
- Rules derived: Loop (fan triangulation, Loop weights, OpenSubdiv numbering: edge list from the first fan triangles then the second ones; inner edges before halves; proven by `r25-type-osd_loop` and `scheme-osd_loop`). Boole layout (two objects with own points, proven by all four r25 cases) and, by hand only, the start-corner rules (Sutherland-Hodgman against the other solid's planes for kept-inside polygons, `r25-intersect`, `r25-a_minus_b` B part; last new point of the cut run for outside polygons, `r25-union`). Parity-pending (no rule found in three hypotheses): numbering of the new points (A part P1..P6 walking faces x+, y+, z+; B part P1,P6,P2,P5,P3,P4) and the face order of union/subtract/without (A z-,x+,y-,z+,y+,x-; B union z-,y-,x+,z+,x-,y+); details in BOOLE.md §5a.
- Deviations: `boole` keeps the welded output for the viewport (existing evaluate tests rely on it); the R25 layout is the separate `boole_objects`. Spec for Loop was written after the code worked (data fit on the first run).
- Files: crates/geometry/src/subdivide.rs (+ tests/subdivide.rs), crates/generators/src/{subdivision,boole,lib}.rs, crates/generators/tests/{subdivision_parity,boole_parity}.rs, docs/specs/{SUBDIVISION,BOOLE,GENERATORS2}.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions / follow-up cases: Boole with B shifted along one axis only (cube at (50,0,0)), B strictly inside A (smaller cube), and B shifted by different amounts per axis (e.g. (30,50,70)); each in union and a_minus_b. They separate new-point numbering and face order from the symmetric geometry of the current case. Also a Loop case on a non-quad input (a default pyramid or a sphere) to confirm the fan order for n-gons.
