# OBJ export settings — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827), title bar "CINEMA 4D R20.059 Studio (RC - R20)" · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-17

Every golden `mesh.obj` is exported with these settings. If you ever change one,
note it in the affected observation file and its `case.toml`.

## Exporter options (as shown)

Opened via File > Export... > "Wavefront OBJ (*.obj)". A file save dialog comes
first, then a dialog titled "OBJ Export". The dialog **resets to the "Default" preset
every time it opens**, so the two non-default values below were set by hand on every
export (selecting them switches the preset label to "Custom"). No preset was saved.

| Option label | Value used | Other values offered |
|---|---|---|
| Presets | Custom (Default + the two changes below) | Default, 3ds Max, Custom; buttons "Save Preset...", "Delete Preset..." (not used) |
| General > Scale | 1 | numeric field |
| General > Scale unit (dropdown next to Scale) | Centimeters (default) | Kilometers, Meters, Centimeters, Millimeters, Micrometers, Nanometers, Miles, Yards, Feet, Inches |
| General > Normals | Vertices (default) | Skip, Vertices |
| General > Texture Coords (UVs) | on (default) | checkbox |
| Material > Material | **No Material** (default is Export Materials) | No Material, Export Name and Default, Export Materials |
| Material > Default Material | White (default) | White, Random Colors |
| Material > Invert Transparency | off (default) | checkbox |
| Optional > Flip X Axis | off (default) | checkbox |
| Optional > Flip Y Axis | off (default) | checkbox |
| Optional > Flip Z Axis | **off** (default is **on**) | checkbox |
| Optional > Swap X and Y Axes | off (default) | checkbox |
| Optional > Swap X and Z Axes | off (default) | checkbox |
| Optional > Swap Y and Z Axes | off (default) | checkbox |
| Optional > Flip Faces | off (default) | checkbox |
| Optional > Flip UVWs | off (default) | checkbox |
| Optional > Export Objects as Groups | off (default) | checkbox |
| Optional > Export Duplicate Vertices | off (default) | checkbox |
| Triangulate | not offered | – |
| Export selected only | not offered (the whole scene is written; only the cube has geometry in every case) | – |

Check that these settings match the calibration: the calibration scene (cube at
10, 20, 30) re-exported with the settings above gave a file **byte-identical** to
`tests/golden/calibration/offset-cube/mesh.obj`. Exported once with the default
preset instead (Flip Z Axis on, Export Materials): every vertex Z was negated
(e.g. `v -90 -80 70` instead of `v -90 -80 -70`), the `vn` list order changed, and
each face's vertex order was reversed. With "No Material" the file still contains
`usemtl default` and no `mtllib` line.

## Scene settings at export time

| Setting | Value |
|---|---|
| Scene/project unit (e.g. cm, m) | Project Settings (Attributes when nothing is selected): Project Scale = 1, unit Centimeters. Fields show "cm". |
| Up axis as shown in the viewport (which axis points up) | Y (green axis, label "Y" in the viewport corner axis icon, points up in the default Perspective view) |
| Any "export scale" / "convert units" option | Exporter: Scale field (1) + unit dropdown (Centimeters). Project Settings also has a "Scale Project..." button (not used). |

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
