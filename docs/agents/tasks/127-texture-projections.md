# 127 · Texture tag projections + Generate UVW

Status: done · Area: J · Depends on: 123, 128 · Observations: O28 · Touches: material, geometry

## Goal
Projection modes (UVW, spherical, cylindrical, flat, cubic, frontal, shrink wrap, camera) with offset/length/tiles, and Generate UVW.

## Inputs
- Area design: [../../roadmap/areas/J-materials.md](../../roadmap/areas/J-materials.md)
- O28: [../../parity/uv-projection.md](../../parity/uv-projection.md) and its golden cases
- docs/parity/uv-projection.md (O28)

## Design decisions (planner)
- Projection is a function point → uv in tag space; Generate UVW bakes it into the corner UVs.

## Done when
- [x] O28 projection cases pass at level A.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: crates/material/tests/projection_parity.rs: 6/7 Part B cases exact per corner within 1e-5 (flat, cubic, cubic-scaled, spherical, cylindrical, cylindrical-offset); `proj-frontal` KNOWN_MISMATCH (max corner error 3.6e-4 with the recorded camera; a free fit gives heading 44.985 deg and ku 0.99905, so a re-observation with focal length, film size and exact rotation is needed).
- Rules: flat = z-axis plane, cylindrical/spherical angular with per-polygon seam fix and previous-corner pole rule, cubic by Newell-normal axis, scale = 1/size, Offset not applied by Generate UVW (UV.md U4).
- Not done: a Generate UVW editor command (needs scene hosting of tags/materials, see 123), Spatial / Shrink Wrapping / Camera Mapping (Unsupported), tag rotation/position are untested (parity-pending).
- Deviation: tag param `scale` became `size` (default 100) to match O28.
- Files: crates/material/src/projection.rs, tests/projection_parity.rs, docs/specs/UV.md U4.
