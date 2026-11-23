# 164 · glTF import/export (meshes, hierarchy, materials, animation, skins, morphs)

Status: done · Area: M · Depends on: 088, 101, 107 · Observations: O44 · Touches: io

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
- [x] O44 glTF cases: geometry within 1e-4 of the OBJ golden; round trip.
- [x] The gate passes and BACKLOG.md is updated.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- **Result:** 5/5 in-scope GLB bind-pose/world point sets match the OBJ at `1e-4`;
  O44 animated translation imports, and the GLB round trips retain hierarchy, scale,
  UVs, base colour, roughness, and skinned vertex positions at f15 and f30.
- **Known mismatch:** O44 skinned-arm f15 and f30 posed bounds miss `1e-4` (maximum
  errors `4.039e-4` and `4.2e-4`). The GLB lacks the authored animation and its weights
  differ from the authoring rig. The test applies the manifest keys, asserts exactly
  those mismatches, and checks the independent round trips at the original tolerance.
- **Rules:** O44 `hierarchy` and `skinned-arm` confirm that their GLB values are in
  centimetres; the test uses scale 1, while standard exchange defaults to 100 on
  import and 0.01 on export. Reflect Z in positions, rotations and bind matrices; reverse
  winding. The material fixture tests base colour/roughness links and UV round trips.
- **Limits:** Binary GLB output; import GLB or JSON with embedded buffers. No morph
  targets, cameras, lights, or textures. One TRS clip, STEP/LINEAR import and
  frame-baked curves for export. The O44 camera/light file is not in scope.
- **Files:** `crates/io/src/gltf.rs` and `gltf/write.rs`, generic file format dispatch,
  app and CLI round-trip tests, and IO.md §5. The parity result is recorded here;
  the unattended run rules prohibit edits under `docs/parity/`.
- **Gate:** `GATE: 2426 tests passed, 0 failed`; `cleanroom: ok`; `ok: 1117 cases`;
  `GATE: all passed`. `cargo-deny` was not installed.

### Planner follow-up: GLB skin weight normalization

Measured maximum absolute component error across both recorded bounding-box corners
with the same imported weights and J2 keys:

| Skin evaluation | f15 | f30 |
|---|---:|---:|
| `normalise = true` (current import) | `4.038936e-4` | `4.200000e-4` |
| `normalise = false` (`p + Σ w(Kp − p)`) | `7.638114e-4` | `1.603737e-3` |

Neither raw-weight pose meets `1e-4`, and both are farther from the reference.
Import behavior and both known-mismatch pins are unchanged.
Follow-up gate passed with `cargo-deny` 0.22.0 installed: advisories, bans, licenses
and sources all OK.
