# Exchange references — observation (O44)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted.

Build each scene in a new document. Export it **once per format the app offers**, through
its documented export call, with that format's **default** options, except:
- the OBJ uses the usual settings (the geometry truth);
- glTF uses binary `.glb` if offered.

Record each exporter's option list with its defaults, once per format.

Output: `tests/golden/exchange/<case>/`:
- `mesh.obj` (always);
- `scene.fbx`, `scene.glb` (or `.gltf` + `.bin`), `scene.abc`, `scene.stl`, `scene.ply`,
  `scene.usd*`: whatever exists in R25.

`case.toml`: `feature = "exchange"`, the scene description in `[params]`, and the export
options per format in `[params.export_<format>]`. Keep each case under 3 MB.

Unblocks: M/164–171. Level B (see `docs/roadmap/areas/M-io.md`).

## Exporter options (fill once per format)

| Format | Option | Default |
|---|---|---|

## Cases

| Case | Scene | Animation |
|---|---|---|
| static-mesh | a cube (segments 2/2/2) and a sphere (segments 16) at (300, 0, 0) | none |
| hierarchy | a Null at (100, 0, 0), rotated H 45°, with a cube child at (0, 100, 0) scaled 2 | none |
| anim-transform | a cube keyed with record: frame 0 at (0, 0, 0), frame 30 at (200, 100, 0) with rotation H 90° and scale 1.5 | 0–30 |
| skinned-arm | the O34 setup (cylinder, 3 joints, explicit weights), with J2 keyed B 0° at 0 and 90° at 30 | 0–30 |
| morph | the O37 sphere with morph A animated 0 % at 0 and 100 % at 30 | 0–30 |
| camera-light | a camera at (0, 200, −600), P −15°, focal 36; an Omni light at (300, 400, −300); a Spot light | none |
| materials | three cubes, each with a different material: red matte, blue glossy, checker texture | none |

## Notes


### static-mesh — batch04
Measured scene values and all scalar public object settings are recorded in the case manifest. Exported OBJ plus FBX, binary glTF, Alembic and STL with read-back options; OBJ uses calibrated settings. PLY/USD not observed: no public format constant/export command match found; no exporter installed. Export file bytes/signatures checked separately; the golden validator validates OBJ, not exchange semantic equivalence.

```text
vertices: 140 (unused 0)
faces: 152 (3-gons 32, 4-gons 120)
bbox min: (-100, -100, -100)
bbox max: (400, 100, 100)
size: (500, 200, 200)
centre: (150, 0, 0)
normals: 120
uvs: 153
```

### hierarchy — batch04
Measured scene values and all scalar public object settings are recorded in the case manifest. Exported OBJ plus FBX, binary glTF, Alembic and STL with read-back options; OBJ uses calibrated settings. PLY/USD not observed: no public format constant/export command match found; no exporter installed. Export file bytes/signatures checked separately; the golden validator validates OBJ, not exchange semantic equivalence.

```text
vertices: 8 (unused 0)
faces: 6 (4-gons 6)
bbox min: (-182.842712, -100, -282.842712)
bbox max: (382.842712, 300, 282.842712)
size: (565.685424, 400, 565.685424)
centre: (100, 100, 0)
normals: 6
uvs: 4
```

### anim-transform — batch04
Measured scene values and all scalar public object settings are recorded in the case manifest. Exported OBJ plus FBX, binary glTF, Alembic and STL with read-back options; OBJ uses calibrated settings. PLY/USD not observed: no public format constant/export command match found; no exporter installed. Export file bytes/signatures checked separately; the golden validator validates OBJ, not exchange semantic equivalence.

```text
vertices: 8 (unused 0)
faces: 6 (4-gons 6)
bbox min: (-100, -100, -100)
bbox max: (100, 100, 100)
size: (200, 200, 200)
centre: (0, 0, 0)
normals: 6
uvs: 4
```

### skinned-arm — batch04
Scene: an editable 106-point arm mesh (40 x 300 x 40, y -150..+150) bound to a three-joint chain J1 > J2 > J3 through a skin deformer and a weight tag with explicit linear weights (the rule, per-point samples and the sha256 of the weight triples are in the case manifest). J2 is keyed on its bank channel, 0 deg at frame 0 and 90 deg at frame 30. `mesh.obj` is the frame-0 (bind) pose; the deformed poses were exported and measured separately, and their bounding boxes and joint angles are recorded in the manifest under `posed_samples`. Measured scene values and all scalar public object settings are recorded in the case manifest. Exported OBJ plus FBX, binary glTF, Alembic and STL with read-back options; OBJ uses calibrated settings. PLY/USD not observed: no public format constant/export command match found; no exporter installed. The golden validator validates OBJ, not exchange semantic equivalence.

```text
vertices: 106 (unused 0)
faces: 112 (3-gons 16, 4-gons 96)
bbox min: (-20, -150, -20)
bbox max: (20, 150, 20)
size: (40, 300, 40)
centre: (0, 0, 0)
normals: 14
uvs: 121
posed bbox f15: (-43.587976, -150, -20) .. (120.208359, 120.207946, 20)
posed bbox f30: (-47.500420, -150, -20) .. (150, 20, 20)
```

Cross-format skin semantics (read back from the exported bytes, not from the app):

- glTF (GLB 2, 16,224 B; the declared length equals the file size; nodes J1, J2, J3 and Arm Mesh; one mesh, one primitive, attributes POSITION/NORMAL/TEXCOORD_0/JOINTS_0/WEIGHTS_0; one skin over 3 joints): the skin and weights survive, but the skin has no name, so the deformer name is dropped, and there is no animation because the export bakes no animation by default.
- FBX (43,344 B, `Kaydara FBX Binary`): the joint names J1, J2, J3 and the deformer name Skin each appear once, with 3 `Cluster`, 9 `Deformer` and 12 `AnimationCurve` records, so joints, weights and both keyframes are carried; the mesh object name Arm Mesh does not appear as plain text.
- Alembic (216,611 B, Ogawa archive `ff 00 01`): geometry only; joint, skin and weight data are absent.
- STL (10,484 B = 84-byte header + 208 triangles x 50 B): the 96 quads are triangulated (96 x 2 + 16 = 208) and every skinning channel is lost.

### morph — batch04
not observed: setting the pose-morph strength track failed (the script aborts silently in the track code; details in /Users/bigman/forge-observer/notes.md)

### camera-light — batch04
Measured scene values and all scalar public object settings are recorded in the case manifest. Exported OBJ plus FBX, binary glTF, Alembic and STL with read-back options; OBJ uses calibrated settings. PLY/USD not observed: no public format constant/export command match found; no exporter installed. Export file bytes/signatures checked separately; the golden validator validates OBJ, not exchange semantic equivalence.

```text
vertices: 0 (unused 0)
faces: 0
bbox: empty
normals: 0
uvs: 0
```

Export contents verified byte-level, per file: `mesh.obj` is the 51-byte WaveFront header only — the scene has no polygon object, so the geometry truth is empty (harness: 0 vertices, 0 faces). `scene.fbx` is binary Kaydara FBX (29,232 B) and carries `Camera`, `Omni`, `Spot` and `FocalLength`. `scene.glb` is GLB v2 (464 B): one JSON chunk of 444 B, no BIN chunk, one camera node at translation (0, 200, 600) with the flip-Z option applied, and no lights and no meshes — R25's glTF export offers no light option, so the two lights are simply absent. Its 12-byte header declares a total length of 472 while the file is 464 bytes, an 8-byte shortfall (the length of the missing BIN chunk header); strict GLB readers that trust the declared length may reject or mis-read the file. `scene.abc` (Ogawa, 1,479 B) carries the camera node but no light nodes. `scene.stl` is the 84-byte binary header plus a zero-triangle count, consistent with the empty mesh.


### materials — batch04
Measured scene values and all scalar public object settings are recorded in the case manifest. Exported OBJ plus FBX, binary glTF, Alembic and STL with read-back options; OBJ uses calibrated settings. PLY/USD not observed: no public format constant/export command match found; no exporter installed. Export file bytes/signatures checked separately; the golden validator validates OBJ, not exchange semantic equivalence.

```text
vertices: 24 (unused 0)
faces: 18 (4-gons 18)
bbox min: (-250, -100, -100)
bbox max: (250, 100, 100)
size: (500, 200, 200)
centre: (0, 0, 0)
normals: 6
uvs: 4
```

Cross-format material semantics (read back from the exported bytes, not from the app):

- OBJ (2,055 B, 24 `v` / 6 `vn` / 4 `vt`, 18 quads): one object per cube (`o Cube_3`, `o Cube_2`, `o Cube_1`), each immediately followed by `usemtl <material name>`. No `mtllib` line and no `.mtl` sidecar is written, so material *names* survive the exchange but colour, reflection and shader settings do not.
- glTF (GLB 2, 5,788 B; header length equals the file size; JSON chunk 3,240 B + BIN chunk 2,520 B; 3 meshes, 3 nodes, 1 primitive each): every primitive is bound to its own material and the binding follows scene order - `Checker Texture` -> Cube 3, `Blue Glossy` -> Cube 2, `Red Matte` -> Cube 1. `baseColorFactor` matches the C4D colour exactly: Red Matte `[1,0,0,1]`, Blue Glossy `[0,0,1,1]`, Checker Texture `[0.8,0.8,0.8,1]`. The reflection switch lands on the metallic/roughness pair: the two materials with reflection enabled export `metallicFactor 0.0` / `roughnessFactor 0.1`, the non-reflective Red Matte exports `1.0` / `1.0`. There are no `images`, `textures` or `samplers`: the Checkerboard shader is flattened to its average base colour, so the procedural pattern is not baked.
- FBX (45,328 B, `Kaydara FBX Binary`): the three material names each appear once, with three `DiffuseColor` properties, 19 `Material` and 6 `LayerElementMaterial` records. `FbxSurfacePhong` appears once and `FbxSurfaceLambert` never; that single literal is the class template in the Definitions section, so all three materials are written as Phong regardless of the reflection flag.
- Alembic (4,364 B, Ogawa archive `ff 00 01`): geometry only. Object and material names are absent as plain text, so material assignment is not recoverable from the archive.
- STL (1,884 B = 84-byte header + 36 triangles x 50 B): the quads are triangulated (18 quads -> 36 triangles) and no material information is carried.

Reading order for a consumer: OBJ gives names + topology, glTF gives names + colour + gloss, FBX gives names + diffuse colour, Alembic gives geometry, STL gives triangles. Only glTF is a lossless carrier for this case's material set.
