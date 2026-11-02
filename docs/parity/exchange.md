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
TODO
