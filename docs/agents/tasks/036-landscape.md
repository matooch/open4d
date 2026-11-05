# 036 · Landscape primitive

Status: done (parity-pending heights) · Area: C · Depends on: 063 · Observations: O18 · Touches: generators, noise

## Goal
Landscape with every observed parameter. The mesh is deterministic per seed. Comparison is statistical unless the exact noise rule is found.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O18: [../../parity/primitives-3.md](../../parity/primitives-3.md) and its golden cases
- docs/parity/primitives-3.md (Landscape notes: deterministic from its settings; size, furrows, sea/plateau levels, multifractal, borders, spherical)

## Design decisions (planner)
- Grid (width × depth segments) with heights from fractal noise (`forge-noise`), with sea and plateau clamping and borders at sea level.
- Parity: the grid topology and x/z positions match exactly. The heights are compared by histogram and by correlation with the golden. Exact heights are parity-pending.

## Done when
- [x] PRIMITIVES3.md §Landscape.
- [x] Tests: topology exact; height statistics within tolerance for all 4 cases.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `crates/generators/src/landscape.rs` (`forge.landscape`, registered in `Primitives`, depends on `forge-noise`), tests `landscape_parity.rs` (3: topology, statistics, continuity in position) and `landscape.rs` (6 unit tests). Parity: topology and x/z positions exact 4/4 (face lists identical, positions within 1e-4); heights statistical 4/4 (mean, std, maximum, 10-bin histogram overlap >= 0.60, correlation >= 0.25, border vertices 0; measured values in PRIMITIVES3.md L4). `UNSUPPORTED` / `KNOWN_MISMATCH`: none. Heights are **parity-pending** (own noise model, level C).
- Rules derived (PRIMITIVES3.md §6): L1 grid `k*n+i`, X fastest, quads `[a, a+n, a+n+1, a+1]` (`landscape/defaults`, `segments`). F1: the field is a function of position, the 20x20 export equals the 100x100 export every 5th point times one constant 1.02489 (all 361 points), so a global normalisation by the grid extremes is the only grid dependence. F2: border mask is a function of the Chebyshev border distance, about `t^2` near 0 (corner ratios 4.26 / 10.3 reject a product of axis masks); ours `sin^2(pi t)`. F3 statistics of the field (mean 0.45 to 0.59, std 0.19 to 0.24, correlation length about a third of the size) fix the noise period `0.3 * size * scale` and the octave weights.
- Height model hypotheses: H1 product-of-axes mask (rejected), H2 Chebyshev `sin^2` mask (kept), H3 grid-extreme normalisation (kept). The reference noise itself is not recoverable from the mesh, so no exact rule; heights compared statistically as the brief allows. Sea/plateau (`clamp`, then subtract sea and rescale), spherical (lon/lat wrap) and orientation are our own readings, never observed.
- Deviations: percent params are stored as fractions (case files hold percent; the test divides by 100); the multifractal offset 1.2 is a tuned free constant (documented); the "Fractal Noise off" case is Multifractal off (O18 note). Not on the command bar.
- Files: crates/generators/src/{landscape.rs (new), lib.rs}, crates/generators/Cargo.toml, Cargo.lock, crates/generators/tests/{landscape.rs, landscape_parity.rs} (new), docs/specs/PRIMITIVES3.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Open data that would settle the pending rules: Landscape with Sea Level 50 % and with Plateau Level 50 % (clamp and rescale rule); Spherical on (mapping, radius); Orientation +Z; Scale 2 and Scale 0.5 (feature size law); Size X/Z 1200 and Size Y 200 (does the pattern scale with size or stay fixed); Rough Furrows 0 / 100 % and Fine Furrows 0 / 100 % (octave weights); Borders At Sea Level off (is the field then unmasked); Segments 4 (smallest grid, exposes the normalisation); several more seeds (seed to field map); a flat 1-D slice at high resolution (e.g. Segments 1000) would show the lattice noise and its octave spacing.
- The last lines of the gate output: `GATE: 1432 tests passed, 0 failed`; python `OK`; `cleanroom: ok`; `ok: 471 cases`; `GATE: all passed`.
