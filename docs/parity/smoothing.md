# Smoothing / normals — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-17
Unblocks: smoothing angle, normal weighting (GEOMETRY.md §3 parity-pending), viewport
smooth shading. **Normals must be exported** (OBJ `vn` lines; `golden info` shows
`normals: N`). Folder: `tests/golden/smoothing/<case>/`.

All exports use [export-settings.md](export-settings.md) (Normals = Vertices, No
Material, Flip Z Axis off), each from File > New with only that one object.

## How smoothing is controlled

| Question | Answer |
|---|---|
| What controls smooth vs faceted shading (object setting, tag, …)? | A **tag** on the object, named "Phong" (shown as a tag icon next to the object in the Objects list, and as a "Phong" tab in the object's Attributes). Its "Tag Properties": **Angle Limit** (checkbox), **Phong Angle**, **Use Edge Breaks** (checkbox). "Basic Properties": Name = "Phong", Layer. A "Delete Tag" button sits under it. |
| Is it present on new primitives by default? | Yes. Cube, Sphere, Plane, Cylinder and Cone all show a "Phong" tab right after creation; cube and cylinder were checked to have Angle Limit on, Phong Angle 40 °, Use Edge Breaks on. |
| Smoothing-angle parameter label, default, range, unit | "Phong Angle", default 40, range 0 to 180 (typed 999 → 180 °, typed -10 → 0 °), unit ° (degrees). It applies while "Angle Limit" (default on) is checked. |
| Can individual edges be marked hard/sharp? How? | The menu Mesh > Normals has "Break Phong Shading", "Unbreak Phong Shading" and "Select Broken Phong Edges" (greyed out for an unconverted primitive cube; not tried). The tag also has a "Use Edge Breaks" checkbox (default on); its effect was not tested. |

## Cases

| Case | Object & settings | `golden info` (incl. normals count) | Notes |
|---|---|---|---|
| cube-default | default cube (200, segments 1), Phong tag defaults (Angle Limit on, 40 °, Use Edge Breaks on) | 8 vertices / 6 faces (4-gons 6) / size (200, 200, 200), centre (0, 0, 0); **normals 6**, uvs 4 | one `vn` per face: (0,0,-1), (1,0,0), (0,0,1), (-1,0,0), (0,1,0), (0,-1,0). Identical to `tests/golden/cube/defaults/mesh.obj` (apart from line endings). |
| cylinder-default | default cylinder: Radius 50, Height 200, Height Segments 1, Rotation Segments 36, Orientation +Y; Caps on, cap Segments 1, Fillet off; Phong tag defaults (40 °) | 74 / 108 (3-gons 72, 4-gons 36) / size (100, 200, 100), centre (0, 0, 0); **normals 39**, uvs 109 | caps vs sides: 36 distinct horizontal side normals (y = 0, e.g. (1,0,0), (0.98480772972107, 0, ±0.17364820837975), …) shared up and down the side, plus (0,1,0) and (0,-1,0) for the caps, plus one `vn 0 0 0` (the 2nd `vn`) that **no face uses**. |
| cube-angle-89 | default cube, Phong Angle 89 (rest default) | 8 / 6 (4-gons 6) / size (200, 200, 200), centre (0, 0, 0); **normals 6**, uvs 4 | same 6 face normals as cube-default |
| cube-angle-91 | default cube, Phong Angle 91 (rest default) | 8 / 6 (4-gons 6) / size (200, 200, 200), centre (0, 0, 0); **normals 8**, uvs 4 | one normal per vertex, each (±0.57735025882721, ±0.57735025882721, ±0.57735025882721), i.e. the corner direction |
| uneven | default cone (Top Radius 0, Bottom Radius 100, Height 200, Height Segments 8, Orientation +Y; Caps on, Cap Segments 1) with **Rotation Segments 3** (the minimum; typing 1 gives 3); Phong tag defaults (40 °) | 32 / 33 (3-gons 3, 4-gons 30) / size (150, 200, 173.205078125), centre (25, 0, 0); **normals 15**, uvs 44 | normals: (0,-1,0) for the bottom cap; one `vn 0 0 0` (2nd entry) used by no face; then 13 side normals that are three face directions repeated with tiny float differences: (0.4850712…, 0.2425356…, ±0.84016805887222) six times each sign group and (-0.97014248371124, 0.24253562092781, 0) once. So each side vertex carries its own face's normal (no averaging across sides). |

## Behaviour notes
- The cube flips from per-face normals (6) at 89 ° to per-vertex averaged normals (8)
  at 91 °: the 90 ° cube edges are smoothed once Phong Angle exceeds 90.
- Even on smoothed surfaces the exporter writes a separate `vn` line per vertex/face
  combination with float noise in the last digits (cone sides), instead of reusing
  exactly equal normals.
- Cylinder and cone files contain an unused `vn 0 0 0` as their second normal.
- Not tested: Angle Limit off, Use Edge Breaks off, and broken Phong edges on an
  editable mesh.
