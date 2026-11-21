# 073 · Matrix object

Status: done · Area: E · Depends on: 065 · Observations: O31 · Touches: clone-engine

## Goal
A Matrix object: a cloner that produces only matrices (no geometry), usable as a distribution source.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O31: [../../parity/clone-generators-2.md](../../parity/clone-generators-2.md) and its golden cases
- docs/parity/clone-generators-2.md (matrix cases)

## Design decisions (planner)
- Shares the cloner modes; produces `CloneData` with no prototypes. Exposes its matrices to Object-mode cloners and to the Tracer.
- Viewport: draw matrices as small axes (079).

## Done when
- [x] O31 matrix cases pass.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `matrix_parity.rs` (2 tests); parity 2/2 exact (`matrix/defaults` 1600 clones, `matrix/linear` 5 clones); no UNSUPPORTED/KNOWN_MISMATCH cases. Particles and Object mode return Unsupported (unit test).
- Rules: Matrix = cloner modes + Transform tab without prototype options; defaults Grid, count (40,1,40), size 10.5 (`matrix/defaults`); Linear as cloner (`matrix/linear`). Parity-pending: Radial/Honeycomb/effectors on a Matrix, O31's mislabelled per-mode counts.
- Deviations: none. Viewport axes are brief 079; Object-mode/Tracer consumers read `matrix_data` (Tracer in 076).
- Files: crates/clone-engine/src/{matrix,node,lib}.rs, crates/generators/src/{cloner,lib}.rs, crates/generators/tests/matrix_parity.rs, crates/ui/icons/matrix.svg, crates/ui/src/icon_table.rs, docs/specs/MOGRAPH.md (M1-M2), STATUS, BACKLOG.
- Open: none.
- Gate: per-crate clippy and tests pass (clone-engine, generators, io, ui, app); full gate run before final commit.
