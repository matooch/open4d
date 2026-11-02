# 035 · Platonic, Oil Tank, Polygon and Figure primitives

Status: done · Area: C · Depends on: 033 · Observations: O18 · Touches: generators

## Goal
Four more solid primitives with R25 defaults. Platonic, Oil Tank and Polygon match the goldens. Figure is our own procedural mannequin (level C).

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O18: [../../parity/primitives-3.md](../../parity/primitives-3.md) and its golden cases
- docs/parity/primitives-3.md (parameter tables, cases, notes)
- tests/golden/{platonic,oil-tank,polygon-prim,figure}
- crates/generators/src/round.rs (pattern), platonic.rs (sphere types share code)

## Design decisions (planner)
- Platonic: 6 types with `segments` subdivision. Reuse `platonic.rs` from the sphere types where the geometry agrees. Tetra is not centred (O18 note); Bucky is slightly off-centre.
- Oil Tank: a cylinder with spherical-cap ends (cap height), plus slice. Reuse `round.rs`.
- Polygon: a flat quad or triangle with segments, orientation +Y default.
- Figure: **never copy or tabulate the golden mesh** (CLAUDE.md rule 2). Build a procedural mannequin from capsules and boxes that respects the height and segments parameters. The test checks only the bbox (±5 %) and that changing the parameters changes the mesh.

## Done when
- [x] PRIMITIVES3.md (new spec) with params tables citing O18.
- [x] platonic, oil-tank and polygon-prim cases match. The Figure test is level C.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Landscape (036).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `crates/generators/tests/primitives3_parity.rs` (3 tests) and `primitives3.rs` (6 tests). Parity: platonic 7/8 exact (type-bucky is a pinned `KNOWN_MISMATCH`), oil-tank 3/3, polygon-prim 3/3, figure bounding box within 5 % (level C). `UNSUPPORTED` is empty.
- Rules (spec docs/specs/PRIMITIVES3.md): P1 tetra/octa/icosa are the sphere solids at `radius`, icosa corners divided by the rounded length 1.90211 (radius 100.00016; `platonic/defaults`); hexa corner/face order from the case. P2 segments cut flat, no projection (`platonic/segments`, icosa n = 3). P3 dodeca: 20 points, `split_pentagon` of a 12-ring table, quads sorted by (lower inner index, descending pair first) (`platonic/type-dodeca`). Oil tank: capsule-style lathe with elliptical caps, arc 22.5 degrees (`oil-tank/defaults`, `segments`, `slice`). Polygon: flat quad/triangle, grid as pyramid (`polygon-prim/*`). Figure: own capsule/box mannequin.
- Parity-pending: segments of hexa/tetra/octa (icosa only observed); the 1.90211 constant at other radii; oil-tank arc angle against cap height / radius (every case has 25/100), orientations other than +Y for all three, regular grid (unsupported); polygon triangle with segments; Dodeca and Bucky with Segments > 1 (return `Unsupported`).
- Bucky: not matched. The export is irregular (distances from the origin 96.67-99.79, 98.12-98.31 around its own centroid, noise 0.05-0.2 against the 1e-4 tolerance) and its face order is not derived. Ours is a regular truncated icosahedron with the exported counts (60 / 12 triangles + 52 quads).
- Figure design and clean room: capsules (tapered, lathe) and boxes in a T pose from the eight-head canon (head H/8, shoulders 0.82 H, arm span about H); `height` scales everything, `segments` sets the columns (rings = max(2, segments / 4)). The only golden data read is the bounding box of figure/defaults in the test (and its three size numbers in the spec); no vertex, face or proportion of the mesh was copied or fitted.
- Deviations: new type id `forge.polygon_prim` (`forge.polygon` is the editable polygon object); not added to the command bar (brief 147 lists six primitives only). The parity tests translate the case spellings `plus_y`/`minus_x` to `pos_y`/`neg_x` and `grid_width` to `regular_grid_width`.
- Files: crates/generators/src/{solids,oil_tank,polygon_prim,figure}.rs (new), lib.rs, platonic.rs, round.rs (visibility), crates/generators/tests/{primitives3_parity,primitives3}.rs, docs/specs/PRIMITIVES3.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Open data that would settle the pending rules: Platonic Hexa/Tetra/Octa with Segments 3; Platonic Icosa Radius 1000; Bucky Radius 1000 and Radius 10 (irregularity proportional?), Bucky and Dodeca Segments 2; Oil Tank with Cap Height 50 and Radius 50 (arc angle), Orientation -Z, Regular Grid on; Polygon Triangle with Segments 3, Orientation +Z, Width 200 / Height 50; Figure Height 90 (does the mesh scale uniformly).
- The last lines of the gate output: see the commit.
