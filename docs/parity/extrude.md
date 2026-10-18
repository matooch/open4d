# Extrude generator — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/extrude/<case>/`.

## Where it lives
Create > Generators > Extrude. The spline (Create > Spline > Circle / Rectangle) is added
first and dragged onto the Extrude. One scene per input type; the circle cases were
exported one after another from the same scene, changing only the listed value (and
resetting the previous one). Attributes tabs: Basic, Coord., Object, Caps, Phong.

Input: a spline primitive as the child (Circle spline or Rectangle spline, defaults:
Circle radius 200 / Rectangle 400 × 400, Plane XY, Intermediate Points Adaptive, Angle 5°;
see [splines.md](splines.md)). Splines don't export, so there is no `input.obj`; the input
is described in `[params.input]`.

**What the export contains:** the result directly, no Current State to Object needed. Groups:
`o Extrude` (the side), then `o Cap_1` and `o Cap_2` when caps are on (one shared vertex
list).

## Parameters (as shown in the app)

Object tab:

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Movement (X, Y, Z) | `movement` | vec3 | (0, 0, 20) | not tested | not tested | cm | extrusion offset |
| Subdivision | `subdivision` | int | 1 | not tested | not tested | – | rows along the extrusion |
| Iso Subdivision | `iso_subdivision` | int | 10 | not tested | not tested | – | viewport isoparms only |
| Flip Normals | `flip_normals` | bool | off | – | – | – | |
| Hierarchical | `hierarchical` | bool | off | – | – | – | |

Caps tab:

| App label | Our key | Type | Default | Notes |
|---|---|---|---|---|
| Start | `cap_start` | enum | Cap | None, Cap, Fillet, Fillet Cap |
| Start > Steps / Radius | `start_steps` / `start_radius` | int / float | 1 / 5 cm | greyed unless a fillet option |
| End | `cap_end` | enum | Cap | None, Cap, Fillet, Fillet Cap |
| End > Steps / Radius | `end_steps` / `end_radius` | int / float | 1 / 5 cm | greyed unless a fillet option |
| Fillet Type | `fillet_type` | enum | Convex | greyed unless a fillet option; options not read |
| Phong Angle | `fillet_phong_angle` | float | 60° | greyed unless a fillet option |
| Hull Inwards / Hole Inwards / Constrain | – | bool | on / off / off | greyed with the defaults |
| Create Single Object | `single_object` | bool | off | |
| Rounding UVW Keep Shape | – | bool | off | greyed |
| Type | `cap_type` | enum | N-gons | Triangles, Quadrangles, N-gons |
| Regular Grid / Width | `regular_grid` / `regular_grid_width` | bool / float | off / 10 cm | |

Enum options (label → our key): caps `None` → `none`, `Cap` → `cap` (default), `Fillet` →
`fillet`, `Fillet Cap` → `fillet_cap`; cap type `Triangles` → `triangles`, `Quadrangles` →
`quadrangles`, `N-gons` → `ngons` (default).

## Cases

| Case | Inputs and parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | Circle spline child (defaults) | 136 / 70 (4-gons 68, 68-gons 2) / size (400, 400, 20), centre (0, 0, 10); normals 70, uvs 204 | the default circle samples to **68 points** (first (200, 0, 0), then counter-clockwise in XY: (199.05, 19.67, 0), …); each cap is one 68-gon |
| movement | Circle child, movement (0, 0, 50) | 136 / 70 (4-gons 68, 68-gons 2) / size (400, 400, 50), centre (0, 0, 25) | |
| subdivision | Circle child, subdivision = 3 | 272 / 206 (4-gons 204, 68-gons 2) / size (400, 400, 20), centre (0, 0, 10); normals 91, uvs 342 | 4 rings of 68 (the caps reuse the end rings) |
| caps-off | Circle child, start and end caps off (None) | 136 / 68 (4-gons 68) / size (400, 400, 20), centre (0, 0, 10) | only `o Extrude` |
| rectangle | Rectangle spline child (defaults) | 8 / 6 (4-gons 6) / size (400, 400, 20), centre (0, 0, 10); normals 6, uvs 10 | rectangle points start at (200, 200, 0) and run clockwise seen from +Z: (-200, 200), (-200, -200), (200, -200) |
| rectangle-caps-ngon | – | (= rectangle) | N-gons is already the default; replaced by the two cases below |
| rectangle-caps-triangles | Rectangle child, cap Type = Triangles (added) | 8 / 8 (3-gons 4, 4-gons 4) / size (400, 400, 20), centre (0, 0, 10) | each cap split into 2 triangles |
| rectangle-caps-quadrangles | Rectangle child, cap Type = Quadrangles (added) | 8 / 6 (4-gons 6) / size (400, 400, 20), centre (0, 0, 10) | same counts as N-gons for a 4-point outline |

## Behaviour notes
- Which parameters are greyed out or hidden by others: the fillet fields (Steps, Radius,
  Fillet Type, Phong Angle, Hull/Hole Inwards, Constrain) are greyed unless Start/End is
  a fillet option; Regular Grid Width is greyed while Regular Grid is off.
- Anything surprising: the Adaptive (5°) circle gives 68 points, not 72; the side and the
  caps are separate `o` groups sharing one vertex list.
