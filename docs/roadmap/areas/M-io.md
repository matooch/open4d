# M · File exchange

## Scope (v1)

- [x] Native `.forge` (RON, versioned) and OBJ import/export (brief 04).
- [ ] **Native format hardening:**
  - a migration test per version;
  - an optional compressed binary encoding (MessagePack + zstd) for big scenes;
  - external references (textures) with relative paths;
  - autosave files.
- [ ] **glTF 2.0** import and export:
  - meshes, normals, UVs; hierarchy; cameras; lights (`KHR_lights_punctual`);
  - PBR materials;
  - animation (TRS, morph targets); skins.
- [ ] **FBX** import (ufbx) and export:
  - meshes, hierarchy, materials;
  - animation curves; skin clusters; blend shapes; cameras; lights.
- [ ] **Alembic** import and export (Ogawa): polymesh, xform, animated topology, cameras,
  curves.
- [ ] **USD:** `.usda` export and import of meshes, xforms, cameras, materials
  (UsdPreviewSurface), skel (v1.1).
- [ ] **STL** (ASCII and binary) and **PLY** import/export.
- [ ] **Unit and axis conversion:** centimetres, left-handed Y-up ⇄ metres, right-handed
  Y-up or Z-up per format, with options.
- [ ] **Import/export dialogs** with options; batch export through the CLI.
- [ ] **No reference-app scene files.** `.c4d` is forbidden by the cleanroom gate. Scenes
  come over only through exported standard formats.

## Observation jobs

| Job | Form | Status |
|---|---|---|
| O44 | [exchange.md](../../parity/exchange.md): R25 exports of 6 reference scenes (static mesh, hierarchy, animated transforms, skinned arm, morph, camera and light) as FBX, glTF, Alembic, STL, PLY (whatever R25 offers); the same scenes as OBJ for geometry truth; scene descriptions in `case.toml` | todo |

## Implementation design

- **`forge-io`:** one module per format. Readers produce an `ImportScene` (an
  engine-neutral IR: nodes, meshes, materials, animation, skins). `import_into(tx, ir)`
  maps the IR onto our node types. Exporters run in reverse.
- **Format specifics:**
  - **FBX:** `ufbx` (C, MIT) vendored and built with `cc` (DECISIONS D9). Export uses our
    own ASCII FBX 7.4 writer, adding binary later.
  - **Alembic:** our own Ogawa container reader/writer plus the AbcGeom schemas, from the
    public spec; tested against O44 files.
  - **USD:** `.usda` text only in v1.
- **Parity, level B:**
  - importing an O44 file must give the same geometry as the O44 OBJ of the same scene
    (positions within 1e-4 after unit conversion);
  - animation curves sampled per frame within 1e-4;
  - round trip: export, import, compare.

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 163 | Native format: migrations, binary encoding, external references | – | – |
| 164 | glTF import/export (meshes, hierarchy, materials, animation, skins, morphs) | 088, 101, 107 | O44 |
| 165 | FBX import (ufbx) | 088, 101 | O44 |
| 166 | FBX export (ASCII 7.4) | 165 | O44 |
| 167 | Alembic (Ogawa) import/export | 088 | O44 |
| 168 | USD `.usda` import/export | 123 | O44 |
| 169 | STL and PLY | – | O44 |
| 170 | Camera, light and material exchange across formats | 047, 134 | O44 |
| 171 | Exchange parity tests (S9) | 164–169 | O44 |
| 172 | Import/export dialogs and CLI batch export | 013, 150 | – |

## Done when
S9 passes. Round trips are lossless for the supported features, and the formats behave the
same on all three OS.
