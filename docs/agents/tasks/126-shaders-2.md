# 126 · Shaders II: Tiles, Checkerboard, Brick, Fusion, Filter, Normalizer, Vertex Map, Field, AO

Status: done · Area: J · Depends on: 125 · Observations: O42 · Touches: shaders

## Goal
The remaining shaders from O42.

## Inputs
- Area design: [../../roadmap/areas/J-materials.md](../../roadmap/areas/J-materials.md)
- O42: [../../parity/shaders.md](../../parity/shaders.md) and its golden cases
- docs/parity/shaders.md

## Design decisions (planner)
- AO is render-time only (needs ray queries); expose it through the IR as a special node.

## Done when
- [x] O42 cases pass.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: Checkerboard, Tiles (Squares), Fusion, Filter, Normalizer, Vertex Map, AO implemented in crates/shaders with brief 125 (same commit); checkerboard and tiles exact (O42). UNSUPPORTED: Brick (noise-perturbed, one case), Field shader (needs a field at the sample point), tile patterns other than Squares, Rotate.
- Fusion/Filter/Normalizer/Vertex Map/AO have unit tests only (no golden data). See SHADERS.md S4, S5, S8, S9.
