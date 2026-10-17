# Subdivision surface — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-17
Unblocks: the subdivision generator. Folder: `tests/golden/subdivision/<case>/`.
Export the **generator's result** (the subdivided mesh), not the input.

Generator: Create > Generators > "Subdivision Surface". Each case: File > New, add the
input primitive, add the generator, drag the input onto the generator in the Objects
list (it becomes its child), select the generator, set values in its Attributes
"Object" tab (tabs: Basic, Coord., Object). Export settings as in
[export-settings.md](export-settings.md).

**What the export contains:** only the subdivided result, as one object named after
the input (`o Cube`, `o Plane`, …). The input is not written separately. No "Current
State to Object" / make-editable step was needed.

**Which level the export uses:** the **editor** level ("Subdivision Editor"). With the
defaults (Editor 2, Renderer 3) a cube exported as 98 vertices / 96 quads, the same as
Editor 2 (cube-2), not the 386 / 384 of level 3. All cases below set only "Subdivision
Editor" and leave "Subdivision Renderer" at 3. `level` in `case.toml` = the editor
value.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Notes |
|---|---|---|---|---|---|---|
| Subdivision Editor | `level_view` | int | 2 | 0 (-1 typed → 0) | 6 (99 typed → 6) | **used by OBJ export** |
| Subdivision Renderer | `level_render` | int | 3 | 0 (-1 typed → 0) | 6 (99 typed → 6) | not used by export; looks greyed out with the OpenSubdiv types |
| Type | `scheme` | enum | Catmull-Clark (N-Gons) | – | – | 6 options, listed below |
| Subdivide UVs (Catmull-Clark types) | `subdivide_uvs` | enum | Standard | – | – | options: Standard, Boundary, Edge |
| Subdivide UVs (OpenSubdiv Catmull-Clark, Adaptive, Loop) | `subdivide_uvs` | enum | Linear, All | – | – | options: Linear, Boundaries; Linear, All |
| Boundary Interpolation (OpenSubdiv Catmull-Clark, Adaptive, Loop) | `boundary` | enum | Edge | – | – | options: Edge, Edge and Corner |
| Triangle Subdivision (OpenSubdiv Catmull-Clark, Adaptive) | `triangle_subdivision` | enum | Catmark | – | – | options: Catmark, Smooth |
| Edge Crease (OpenSubdiv Catmull-Clark, Adaptive, Loop) | `edge_crease` | enum | Chaikin | – | – | options: Uniform, Chaikin |
| Adaptive (GPU) Tessellation Level (OpenSubdiv Catmull-Clark (Adaptive) only) | `adaptive_level` | int | 2 | not tested | not tested | |

The fields shown depend on Type: Catmull-Clark and Catmull-Clark (N-Gons) show Type,
Subdivision Editor, Subdivision Renderer, Subdivide UVs. OpenSubdiv Bilinear shows only
Type, Subdivision Editor, Subdivision Renderer.

Type options (label → our key), dropdown order: `Catmull-Clark` → `catmull_clark`,
`Catmull-Clark (N-Gons)` → `catmull_clark_ngons` (default),
`OpenSubdiv Catmull-Clark` → `osd_catmull_clark`,
`OpenSubdiv Catmull-Clark (Adaptive)` → `osd_catmull_clark_adaptive`,
`OpenSubdiv Loop` → `osd_loop`, `OpenSubdiv Bilinear` → `osd_bilinear`.

## Cases

Input meshes are default primitives placed under the generator; Type is the default
Catmull-Clark (N-Gons) unless the case says otherwise.

| Case | Input & settings | `golden info` (vertices / faces / size) | Notes |
|---|---|---|---|
| cube-1 | default cube, level 1 | 26 / 24 (4-gons 24) / size (200, 200, 200), centre (0, 0, 0) | normals 24, uvs 9 |
| cube-2 | default cube, level 2 | 98 / 96 (4-gons 96) / size (175.69444274902344, 175.69444274902344, 175.69444274902344), centre (0, 0, 0) | normals 98, uvs 25. Identical counts/bbox to the default-settings probe (Editor 2 / Renderer 3). |
| cube-3 | default cube, level 3 | 386 / 384 (4-gons 384) / size (169.8350830078125, 169.8350830078125, 169.8350830078125), centre (0, 0, 0) | |
| plane-1 | default plane (400 × 400, 20 × 20 segments, +Y), level 1 | 1681 / 1600 (4-gons 1600) / size (400, 0, 400), centre (0, 0, 0) | open boundary rule: the bbox stays ±200, i.e. the outer border (at least its extreme points) is not pulled in |
| tris-1 | default pyramid (Size 200 / 200 / 200, Segments 1, +Y), level 1. `input.obj` = the pyramid alone: 5 / 5 (3-gons 4, 4-gons 1), size (200, 200, 200), centre (0, 0, 0) | 18 / 16 (4-gons 16) / size (133.33334350585938, 116.66666412353516, 133.33334350585938), centre (0, -41.66666793823242, 0) | triangle rule: every triangle and the quad became quads only (4 tris × 3 + 1 quad × 4 = 16). bbox (-66.67, -100, -66.67)..(66.67, 16.67, 66.67). |
| ngon-1 | default cylinder (Radius 50, Height 200, Height Segments 1, Rotation Segments 36, Caps on with cap Segments 1), level 1. `input.obj` = the cylinder alone: 74 / 108 (3-gons 72, 4-gons 36), size (100, 200, 100), centre (0, 0, 0) | 362 / 360 (4-gons 360) / size (99.62019348144533, 200, 99.62019348144533), centre (0, 0, 0) | No default primitive exports a face with more than 4 corners (checked: Cylinder, Disc, Polygon, Oil Tank, Tube, Pyramid, Platonic); the cylinder caps are written as 36-triangle fans. 360 = 36 × 4 side quads + 2 × 36 × 3 cap quads, i.e. the caps were subdivided as 72 separate triangles. |
| scheme-catmull_clark | default cube, level 2, Type Catmull-Clark | 98 / 96 (4-gons 96) / size (175.69444274902344, 175.69444274902344, 175.69444274902344), centre (0, 0, 0) | normals 98, uvs 25; same counts and bbox as cube-2 |
| scheme-osd_catmull_clark | default cube, level 2, Type OpenSubdiv Catmull-Clark, other fields default | 98 / 96 (4-gons 96) / size (175.69444274902344, 175.69444274902344, 175.69445037841797), centre (0, 0, -0.000003814697265625) | normals 98, uvs 25 |
| scheme-osd_catmull_clark_adaptive | default cube, level 2, Type OpenSubdiv Catmull-Clark (Adaptive), other fields default (Adaptive level 2) | 8 / 6 (4-gons 6) / size (200, 200, 200), centre (0, 0, 0) | **the export is the unsubdivided cube** (normals 6, uvs 4), although the viewport shows a smooth shape |
| scheme-osd_loop | default cube, level 2, Type OpenSubdiv Loop, other fields default | 98 / 192 (3-gons 192) / size (179.0906982421875, 179.0906982421875, 181.4453125), centre (0, 0, 0) | all triangles; note Z extent differs from X/Y (±90.72265625 vs ±89.54534912109375) |
| scheme-osd_bilinear | default cube, level 2, Type OpenSubdiv Bilinear | 98 / 96 (4-gons 96) / size (200, 200, 200), centre (0, 0, 0) | flat: shape stays a 200 cube, only split (normals 6) |

## Behaviour notes
- The OBJ export uses the Subdivision Editor level, not the Renderer level.
- Every export holds one object only (the result, named after the input child).
- The default type accepts triangles and turns each triangle into 3 quads.
- Cylinder caps reach the generator (and the exporter) as triangle fans, so the
  "n-gon" case could not exercise a real n-gon face from a default primitive.
- OpenSubdiv Catmull-Clark (Adaptive) exports the input mesh unsubdivided.
- Not tested: the Subdivide UVs / Boundary / Triangle / Crease alternatives (each
  case kept them at default), and levels 4–6.
