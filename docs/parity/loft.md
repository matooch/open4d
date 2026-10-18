# Loft generator — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/loft/<case>/`.

## Where it lives
Create > Generators > Loft. The circles are created and set up first (Plane = XZ in their
Object tab, position in Coord.), then dragged onto the Loft (order: Circle, Circle.1, and
for `three` a third Circle appended last). All four cases were exported from one scene,
changing only the listed values in turn and restoring them afterwards. Attributes tabs:
Basic, Coord., Object, Caps, Phong.

Inputs: two Circle splines as children: the first radius 200 at (0, 0, 0), the second
radius 100 at (0, 200, 0), both in the XZ plane (Intermediate Points Adaptive 5°). No
`input.obj` (splines don't export).

**What the export contains:** the result directly: `o Loft` (side), `o Cap_1`, `o Cap_2`
(one shared vertex list). No Current State to Object needed.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Mesh Subdivision U | `mesh_subdivision_u` | int | 30 | not tested | not tested | – | around each section; the closed ring has U − 1 = 29 points (seam welded) |
| Mesh Subdivision V | `mesh_subdivision_v` | int | 10 | not tested | not tested | – | rows between consecutive sections (Subdivision per Segment on) |
| Isoparm Subdivision U | `isoparm_subdivision_u` | int | 10 | not tested | not tested | – | viewport only |
| Organic Form | `organic_form` | bool | off | – | – | – | |
| Loop | `loop` | bool | off | – | – | – | |
| Adapt UV | `adapt_uv` | bool | on | – | – | – | |
| Subdivision per Segment | `subdivision_per_segment` | bool | on | – | – | – | |
| Linear Interpolation | `linear_interpolation` | bool | off | – | – | – | |
| Flip Normals | `flip_normals` | bool | off | – | – | – | |
| Caps tab | `cap_start`, `cap_end`, `cap_type`, … | – | Start Cap, End Cap, **Type Triangles** | – | – | – | same fields as Extrude's Caps tab, but the cap Type default here is Triangles |

## Cases

| Case | Inputs and parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | circles as above | 319 / 344 (3-gons 54, 4-gons 290) / size (398.7476348876953, 200, 399.36822509765625), centre (0.6261825561523438, 100, 0); normals 291, uvs 359 | 11 rings × 29 points at Y 0, 20, …, 200 (first vertex (200, 0, 0)); each cap = 27 triangles; the 29-point ring does not reach ±200 in X/Z exactly |
| mesh-subdivision | mesh subdivision U = 7, V = 3 | 24 / 26 (3-gons 8, 4-gons 18) / size (400, 200, 400), centre (0, 100, 0) | 4 rings × 6 points; caps 4 triangles each |
| three | a third circle radius 150 at (0, 400, 0) | 609 / 634 (3-gons 54, 4-gons 580) / size (398.7476348876953, 400, 399.36822509765625), centre (0.6261825561523438, 200, 0) | 21 rings (10 per segment) |
| caps-off | start and end caps off (None) | 319 / 290 (4-gons 290) / size (398.7476348876953, 200, 399.36822509765625), centre (0.6261825561523438, 100, 0) | only `o Loft` |

## Behaviour notes
- Which parameters are greyed out or hidden by others: none on the Object tab; Caps tab
  as in Extrude.
- Anything surprising: the sections are resampled to Mesh Subdivision U points (not the
  spline's own 68 Adaptive points); a closed section gets U − 1 distinct points. Caps are
  triangulated by default (fan-like, U − 3 triangles).
