# 074 · Fracture and Voronoi Fracture

Status: done (Fracture; Voronoi blocked) · Area: E · Depends on: 065, 033 · Observations: O31 · Touches: clone-engine, geometry

## Goal
Fracture (explode segments, connected, sorted) and Voronoi Fracture (point sources, seeds, inner faces, offset).

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O31: [../../parity/clone-generators-2.md](../../parity/clone-generators-2.md) and its golden cases
- docs/parity/clone-generators-2.md (fracture cases)
- forge-geometry::boolean (winding numbers, robust predicates)

## Design decisions (planner)
- Fracture: split children by connectivity (polygon islands) into clones; ordering rules from O31.
- Voronoi: cells by clipping the mesh by bisector planes of the seed points, inner faces capped and triangulated (033 rules). Point sources: distribution source points (count, seed).
- Parity: geometric (Hausdorff, total volume, cell count); exact positions only if seeds reproduce.

## Done when
- [ ] Fracture cases match; Voronoi cell counts and total volume within 1 %.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `fracture.rs` (2 tests); parity 2/2 exact (`explode-segments`, `plain`). Voronoi Fracture blocked: the three cases were not observed (no meshes), so it is not implemented or registered; design and the data that would settle it are in spec F2.
- Rules: spec F1 (islands, pivot at bbox centre parity-pending, effector child inert). `Unsupported`: Explode Segments & Connect.
- Deviations: none.
- Files: crates/clone-engine/src/{fracture,node,lib}.rs, crates/generators/src/{fracture,lib}.rs, tests/fracture.rs, icons/fracture.svg, icon_table.rs, spec, STATUS, BACKLOG.
- Open: a baked Voronoi result; the connector geometry of the Connect mode.
