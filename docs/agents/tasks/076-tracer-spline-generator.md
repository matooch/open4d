# 076 · Tracer and the clone-toolset spline generator

Status: done (partial) · Area: E · Depends on: 065, 028 · Observations: O31, O50 · Touches: clone-engine, generators

## Goal
Tracer (connect objects / trace paths of vertices, modes and interval) and the spline generator (simple, spline, turtle modes).

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O31: [../../parity/clone-generators-2.md](../../parity/clone-generators-2.md) and its golden cases
- O50: [../../parity/clone-toolset-3.md](../../parity/clone-toolset-3.md) and its golden cases
- docs/parity/clone-generators-2.md, clone-toolset-3.md

## Design decisions (planner)
- Tracer connect mode is deterministic: polyline through clone/object positions in order. Trace paths needs frame history: keep it in the per-frame cache.
- Spline generator turtle mode: an L-system interpreter (own design, rules from the public manual), deterministic.

## Done when
- [ ] O31/O50 tracer and spline-generator cases match.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `tracer_parity.rs` (3 tests); parity 2/5 exact (`clone-spline/simple`, `spline-mode`). `KNOWN_MISMATCH`: `tracer/vertices` and `tracer/interval` (structure exact, positions off by up to 5.2e-4 / 9.8e-4: the reference reads 10.001106 vs 10.000106 at the same frame in the two cases). `UNSUPPORTED`: `clone-spline/turtle` (default premise, rules, angle and length not recorded).
- Rules (spec MOGRAPH.md T1-T3, S1): see spec. Parity-pending: Connect All, Sample Step phase.
- Deviations: Extrude of an open multi-loop spline no longer errors on caps and winds all loops outward (shared `spline_gens.rs`); `evaluate_spline_gen` takes the eval context; generators now depends on forge-anim (exact-time track reads).
- Files: crates/generators/src/{tracer,clone_spline,splines,spline_gens,lib}.rs, Cargo.toml, tests/tracer_parity.rs, ui icons + icon_table, spec, STATUS, BACKLOG.
- Open: Turtle parameter table with default strings; a re-observation of the Tracer cases; Connect All Objects.
