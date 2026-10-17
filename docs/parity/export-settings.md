# OBJ export settings — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO

Every golden `mesh.obj` is exported with these settings. If you ever change one,
note it in the affected observation file and its `case.toml`.

## Exporter options (as shown)

| Option label | Value used | Other values offered |
|---|---|---|
| TODO | | |

## Scene settings at export time

| Setting | Value |
|---|---|
| Scene/project unit (e.g. cm, m) | TODO |
| Up axis as shown in the viewport (which axis points up) | TODO |
| Any "export scale" / "convert units" option | TODO |

## Calibration case (do this first)

Export a default cube **moved to position x = 10, y = 20, z = 30** (no rotation).
Folder: `tests/golden/calibration/offset-cube/`.

- `golden info` prints `centre:`, which should equal the position you typed, possibly scaled
  and/or with one axis negated. Record what you see:

| Typed position | Exported bbox centre | Conclusion (scale factor, flipped axis) |
|---|---|---|
| (10, 20, 30) | (10, 20, 30) | scale 1, no flipped or swapped axis. Faces are wound counter-clockwise seen from outside (right-hand rule), matching the exported `vn` normals: `flip_z = false`. |

From this we set `[import] scale` and `flip_z` for **all** cases. If the conclusion is
anything other than a uniform scale plus an optional Z flip (e.g. Y and Z swapped),
write it down and tell me: the harness import step gets extended before more cases.

Its `case.toml` uses `feature = "calibration"` and
`observation = "docs/parity/export-settings.md"`.

## Calibration result (recorded)

`golden info tests/golden/calibration/offset-cube/mesh.obj`:

```
vertices: 8 (unused 0)
faces: 6 (4-gons 6)
bbox min: (-90, -80, -70)
bbox max: (110, 120, 130)
size: (200, 200, 200)
centre: (10, 20, 30)
normals: 6
uvs: 4
```

- **All cases use `[import] scale = 1.0`, `flip_z = false`.**
- The exporter writes one `vn` per face (6 for a cube), all in `f v/vt/vn` form, and
  a 4-entry UV set; vertices first, then normals, UVs, then faces.
- Side observation for [cube.md](cube.md): the default cube is 200 × 200 × 200.
