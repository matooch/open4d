# 164 · glTF import/export (meshes, hierarchy, materials, animation, skins, morphs)

Status: waiting: O44 · Area: M · Depends on: 088, 101, 107 · Observations: O44 · Touches: io

## Goal
glTF 2.0 import and export.

## Inputs
- Area design: [../../roadmap/areas/M-io.md](../../roadmap/areas/M-io.md)
- O44: [../../parity/exchange.md](../../parity/exchange.md) and its golden cases
- docs/parity/exchange.md (O44)

## Design decisions (planner)
- `gltf` crate; `ImportScene` IR; unit conversion cm ↔ m; handedness conversion (left-handed Y-up ↔ right-handed).
- **Scope now (planner, 2026-11-22):** meshes with UVs (CornerUvs, brief 196), hierarchy and
  transforms, materials through the hosted material list (brief 194, Standard material ↔ glTF
  PBR metallic-roughness, base colour and roughness only), node TRS animation (tracks, brief 088)
  and skins (brief 101, joints + weights + inverse bind matrices). **Out of scope:** morph
  targets, because pose morph (brief 107) is blocked on O37; return `Unsupported` for them and
  note it in IO.md.
- Parity data: O44 `tests/golden/exchange/` (the skinned arm and its GLB export). Compare the
  imported mesh and the frame-15/30 poses to the existing exchange cases with the existing
  tolerances; round-trip our own export → import exactly.

## Done when
- [ ] O44 glTF cases: geometry within 1e-4 of the OBJ golden; round trip.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added; parity N/M (`UNSUPPORTED` / `KNOWN_MISMATCH` with reasons).
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations from the design decisions, and why.
- Files changed.
- Open questions or data that would settle them (the reviewer turns these into follow-up forms).
- The last lines of the gate output.
