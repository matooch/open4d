# Instance object — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/instance/<case>/`.

## Where it lives
Create > Modeling > Instance. With the cube selected when it is created, the Instance
is named "Cube Instance", placed above the cube in the Objects list (not as a child),
and its Reference Object is already set to the cube. Attributes tabs: Basic, Coord.,
Object.

Input: a default Cube at (0, 0, 0) as the reference object; the Instance at (300, 0, 0).

**What the export contains / how mesh.obj was made:** the instance cannot be exported
alone. Disabling the reference cube (its green check) also hides the instance, and the
exporter always writes the whole scene. So the full scene was exported (`scene.obj`,
kept in the folder): it holds two groups, first `o Cube` = **the instance** (8 vertices at
x 200 … 400, i.e. a full copy of the cube's mesh moved to the instance position) and then
`o Cube_1` = the reference cube at the origin (normals and UVs shared with the first
group). `mesh.obj` is the first group cut out of `scene.obj` unchanged (header, its 8
`v`, the 6 `vn`, 4 `vt` and 6 `f` lines). `input.obj` is the default cube
(`tests/golden/cube/defaults/mesh.obj`, identical to the `Cube_1` group).

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Reference Object | `reference` | link | the selected object at creation (here Cube) | – | – | – | |
| Instance Mode | `instance_mode` | enum | Instance | – | – | – | options: Instance, Render Instance, Multi-Instance |

Enum options (label → our key): `Instance` → `instance` (default), `Render Instance` →
`render_instance`, `Multi-Instance` → `multi_instance`.

## Cases

| Case | Inputs and parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | instance of the cube, instance at (300, 0, 0); export the instance alone | 8 / 6 (4-gons 6) / size (200, 200, 200), centre (300, 0, 0); normals 6, uvs 4 | same vertex and face order as the default cube, offset by +300 in X; exported under the reference's name `Cube` (the real cube becomes `Cube_1`) |

## Behaviour notes
- Which parameters are greyed out or hidden by others: none seen.
- Anything surprising: the OBJ writes an instance as a full mesh copy, named after its
  reference object, and renames the reference with a `_1` suffix when it comes later in
  the scene. A disabled reference disables the instance too.
