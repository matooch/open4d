# Lathe generator — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/lathe/<case>/`.

## Where it lives
Create > Generators > Lathe; the spline is created first, set up, then dragged onto the
Lathe. The rectangle cases share one scene (values changed in turn); the circle case
is a new scene. Attributes tabs: Basic, Coord., Object, Caps, Phong.

Input: a Rectangle spline (width 100, height 200) as the child, moved to position
(150, 0, 0), in the default plane (XY; Intermediate Points Adaptive 5°). Circle case: a
Circle spline radius 50 at (150, 0, 0), plane XY. No `input.obj` (splines don't export).

**What the export contains:** the result directly as `o Lathe` (plus `o Cap_1`,
`o Cap_2` when the angle is below 360°). No Current State to Object needed. The profile
is rotated around the **Y axis**.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Angle | `angle` | float | 360 | not tested | not tested | ° | |
| Subdivision | `subdivision` | int | 24 | not tested | not tested | – | rotation segments |
| Isoparm Subdivision | `isoparm_subdivision` | int | 4 | not tested | not tested | – | viewport only |
| Movement | `movement` | float | 0 | not tested | not tested | cm | rise along Y over the sweep |
| Scaling | `scaling` | float | 100 | not tested | not tested | % | |
| Flip Normals | `flip_normals` | bool | off | – | – | – | |
| Caps tab | `cap_start`, `cap_end`, `cap_type`, … | – | Start Cap, End Cap, … | – | – | – | same layout and defaults as Extrude's Caps tab (see [extrude.md](extrude.md)); only matter for angles < 360 |

## Cases

| Case | Inputs and parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | rectangle child as above | 96 / 96 (4-gons 96) / size (400, 200, 400), centre (0, 0, 0); normals 26, uvs 125 | 24 × 4 profile points (rectangle starts at (200, 100, 0), then (100, 100, 0), (100, -100, 0), (200, -100, 0)); closed, no seam duplicates |
| segments | rotation segments (Subdivision) = 7 | 28 / 28 (4-gons 28) / size (380.1937713623047, 200, 389.9711608886719), centre (9.903114318847656, 0, 0); normals 16, uvs 40 | |
| angle | angle = 270 | 104 / 100 (3-gons 4, 4-gons 96) / size (400, 200, 400), centre (0, 0, 0); normals 40, uvs 125 | 25 × 4 points; the two open ends get caps (`Cap_1`, `Cap_2`), each 2 triangles |
| circle | Circle spline child (radius 50) at (150, 0, 0) instead | 1632 / 1632 (4-gons 1632) / size (400, 100, 400), centre (0, 0, 0); normals 1632, uvs 1725 | 24 × 68: the radius-50 circle also samples to 68 points (Adaptive 5°) |

## Behaviour notes
- Which parameters are greyed out or hidden by others: none on the Object tab; Caps tab
  as in Extrude.
- Anything surprising: Subdivision 24 over 360° gives 24 profile copies (the closing copy
  is welded); over 270° it gives 25 copies plus end caps. A rectangle cap (4 points) is
  written as two triangles here, unlike Extrude's default N-gon cap.
