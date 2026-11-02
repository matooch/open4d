# UV coordinates and projection — observation (O28)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted. The usual OBJ settings already export UVs (Original), so every golden
mesh carries `vt` lines.

Folder `tests/golden/uv/<case>/mesh.obj`, with `feature = "uv"` and
`observation = "docs/parity/uv-projection.md"`. `[params]` holds the object, its
parameters, and the projection settings.

Unblocks: P4 (UV tools, texture tags) and the UVs of the primitives we already build.

## Part A: the primitives' own UVs

These cases are the primitive with its default settings. They check that our UV layout
matches the reference.

| Case | Object |
|---|---|
| cube | Cube, defaults |
| cube-segments | Cube, segments 2/3/4 |
| sphere | Sphere, defaults |
| cylinder | Cylinder, defaults |
| cone | Cone, defaults |
| torus | Torus, defaults |
| plane | Plane, defaults |
| disc | Disc, defaults |
| capsule | Capsule, defaults |
| tube | Tube, defaults |

## Part B: projection

Use a sphere made editable. Make a material through its menu command and apply it, which
adds a texture tag. Set the tag's projection, then run **Generate UVW Coordinates** (the
command that bakes a tag's projection into the UVW tag). Export.

| Case | Projection |
|---|---|
| proj-flat | Flat |
| proj-cubic | Cubic |
| proj-spherical | Spherical |
| proj-cylindrical | Cylindrical |
| proj-frontal | Frontal (record the camera) |
| proj-cubic-scaled | Cubic, tag scale (50, 50, 50) |
| proj-cylindrical-offset | Cylindrical, offset U 25 % |

If Generate UVW can't be run from the script, record the reason and skip Part B.

## Notes
TODO
