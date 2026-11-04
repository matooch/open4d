# Caps and bevel tab (Extrude, Lathe, Loft, Sweep) — observation (O56)

Status: done · App version: R25 (25010) · OS: macOS (Darwin 25.6.0) · Observer: Claude (unattended O56 session) · Date: 2026-11-04
Bridge: MCP plugin 0.2.0. Method: scripted. Every case is an Extrude of a default Rectangle
spline, Offset 100, **unless the row says otherwise**; new document per case, mesh OBJ export
with the standard export settings (no [export-settings.md](export-settings.md) deviation).

Output: `tests/golden/caps-bevel/<case>/mesh.obj`, with `feature = "caps-bevel"`. `case.toml`
holds every parameter on the generator's Object, Caps and Bevel sections plus the
Polygon/Edge Selections checkboxes, and the input spline's parameters in `[params.input]`.

Unblocks: C/044, 045.

## Tabs (fill once)

The Caps tab and the Bevel controls are **one tab in R25** (labelled Caps), shared by
Extrude, Lathe, Loft and Sweep — O4 confirmed; both dumps matched. R25 has **no Start/End
Fillet enum** (R20's `cap_start`/`cap_end` are gone): just two Cap checkboxes and bevel
sections.

Caps tab (label → our key, default, options):

| App label | Our key | Type | Default | Notes |
|---|---|---|---|---|
| Caps Type | `caps_type` | enum | N-gon | Triangles, Quadrangles, N-gon, Delaunay, Regular Grid |
| Size | `caps_grid_size` | float | 10 cm | greyed unless Regular Grid (Regular Grid case sets 20) |
| Density | `caps_density` | float | 0.2 | greyed at the default; belongs to Delaunay (not read while visible) |
| Quad Dominant | `quad_dominant` | bool | off | greyed at the default |
| Start Cap | `start_cap` | bool | on | |
| End Cap | `end_cap` | bool | on | |
| Separate Bevel Controls | `separate_bevel_controls` | bool | off | off: one "Both Bevels" section; on: an "End Bevel" section with the same fields appears |
| Bevel Outside | `bevel_outside` | bool | off | see `bevel-inner` |
| Break Phong Rounding | `break_phong_rounding` | bool | on | |
| Avoid Self Intersections | `avoid_self_intersections` | bool | on | |

Bevel section ("Both Bevels"; the hidden "End Bevel" copy has the same fields):

| App label | Our key | Type | Default | Notes |
|---|---|---|---|---|
| Bevel Shape | `bevel_shape` | enum | Round | **Round, Solid, Curve, Step — no Chamfer and no Concave option** |
| Size | `bevel_size` | float | 0 cm | |
| Segments | `bevel_segments` | int | 3 | min 1, max 100; the field rejects a fraction (typed via script) |
| Extended Shape | `bevel_extended_shape` | bool | off | |
| Height | `bevel_height` | float | 0 cm | |
| Shape Depth | `bevel_shape_depth` | float | 1.0 | |
| Curve | – | spline | – | greyed unless Shape Curve |

Object tab, Extrude, R25 (replaces R20's Movement row): Direction enum `Auto` (default), `X`,
`Y`, `Z`, `Custom`, `Absolute` + Offset 100 cm (default); a hidden Movement vector (0, 0, 100)
tracks/serves the Custom direction. Subdivision 1, Isoparm Subdivision 10, Flip Normals and
Hierarchical off, as in R20.

Polygon Selections / Edge Selections (bottom of the tab): Shell, Start Cap, End Cap, Start
Bevel, End Bevel (edges: Start/End Shell, Start Cap, End Cap, Start/End Step) checkboxes, all
off; recorded in `case.toml` as `polygon_selection_*` / `edge_selection_*`.

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 8 / 6 (4-gons 6) / size (400, 400, 100), centre (0, 0, 50); normals 6, uvs 10 | R25 Offset 100 (R20 form used Movement 20) |
| direction-x | Direction X | 8 / 6 (4-gons 6) / size (500, 400, 0), centre (50, 0, 0); normals 3, uvs 10 | extrudes along +X (X −200..300) |
| direction-custom | Direction Custom, Movement (0, 50, 100) | 8 / 6 (4-gons 6) / size (400, 444.72, 89.44); normals 6, uvs 10 | **the Movement is normalized and Offset is the length**: bbox delta (0, 44.72, 89.44) = unit(0, 0.447, 0.894) × 100 |
| caps-triangles | Caps Type Triangles | 8 / 8 (3-gons 4, 4-gons 4), size (400, 400, 100); normals 8, uvs 10 | each rectangular cap = 2 triangles |
| caps-quads | Caps Type Quadrangles | 8 / 6 (4-gons 6); normals 6, uvs 10 | same counts as N-gon for a rectangle |
| caps-ngon | Caps Type N-gon | 8 / 6 (4-gons 6); normals 6, uvs 10 | **N-gon is the R25 Caps Type default** |
| caps-delaunay | Caps Type Delaunay | 10 / 12 (3-gons 8, 4-gons 4); normals 6, uvs 11 | each cap triangulated into 4 tris (Density 0.2 kept at default) |
| caps-grid | Caps Type Regular Grid, Size 20 | 730 / 1452 (3-gons 1448, 4-gons 4); normals 8, uvs 371 | 20×20 grid per cap |
| start-only | End Cap off (Start Cap on) | 8 / 5 (4-gons 5); normals 5, uvs 10 | |
| bevel-chamfer | Bevel Shape Step, Size 10, Segments 1 | 24 / 22 (4-gons 22) / size (400, 400, 120); normals 12, uvs 14 | R25 has no Chamfer option; Step with 1 segment is the 45° chamfer |
| bevel-round | Bevel Shape Round, Size 10, Segments 3 | 32 / 30 (4-gons 30) / size (400, 400, 120); normals 38, uvs 22 | |
| bevel-concave | Bevel Shape Solid (the third option), Size 10, Segments 3 | 56 / 54 (4-gons 54) / size (400, 400, 120); normals 10, uvs 26 | R25 has no Concave option; Solid is the third shape (0 Round, 1 Solid, 2 Curve, 3 Step) |
| bevel-step | Bevel Shape Step, Size 10, Segments 2 | 40 / 38 (4-gons 38) / size (400, 400, 120); normals 12, uvs 18 | |
| bevel-inner | Bevel Shape Round, Size 10, Segments 3, Bevel Outside on | 32 / 30 (4-gons 30) / size (420, 420, 120); normals 38, uvs 22 | **Bevel Outside on grows the bevel outwards** (bbox ±210 instead of ±200) with the same face counts |
| lathe-caps | Lathe of a default Rectangle at x 300, Angle 180° (caps on, default type) | 132 / 130 (4-gons 130) / size (1000, 400, 500), centre (0, 0, 250); normals 69, uvs 165 | |
| sweep-caps-bevel | Sweep of a default Circle profile on an open Arc path (0°–270°), caps on, Bevel Round Size 5 (Segments 3) | 1548 / 1514 (4-gons 1512, 36-gons 2); normals 1752, uvs 1511 | the two cap n-gons are the 36-gons |
| tri-ellipse-loft | Loft of two Circles as ellipses (Radius 200, Radius Y 100), caps on, Caps Type Triangles | 80 / 92 (3-gons 28, 4-gons 64) / size (400, 400, 0); normals 2, uvs 98 | |
| tri-loft-rotated | The same Loft, second section rotated H 30°, Caps Type Triangles | 80 / 92 (3-gons 28, 4-gons 64) / size (400, 400, 200); normals 96, uvs 98 | |
| tri-slice-orient-x | default Cylinder, Orientation +X, Slice on (0°–270°), made editable | 87 / 106 (3-gons 42, 4-gons 64) / size (200, 99.76, 99.76); normals 31, uvs 115 | R25 cylinder defaults differ from R20 (see Notes); exported from the editable Polygon object |
| tri-sweep-open | Sweep of a default Rectangle profile on an open Arc path (0°–270°), Caps Type Triangles | 148 / 148 (3-gons 4, 4-gons 144) / size (800.77, 800.77, 400); normals 102, uvs 185 | |
| tri-extrude-ngon | Extrude of a default n-Side spline with Sides 6, Offset 100, Caps Type Triangles | 12 / 14 (3-gons 8, 4-gons 6); normals 13, uvs 20 | |
| tri-dodeca-seg2 | Platonic Dodecahedron, Segments 2, made editable | 74 / 96 (3-gons 48, 4-gons 48) / size (186.83 cube); normals 60, uvs 113 | exported from the editable Polygon object |

## Notes
- **Caps Type default is N-gon** (the form's guess confirmed); Start Cap and End Cap are plain
  checkboxes, both on — the R20 Start/End fillet enum is gone, filleting is now the Bevel
  section (Size/Solid etc.).
- **Direction Custom treats Movement as a direction**: it is normalized and Offset gives the
  length; the raw Movement (0, 50, 100) then means "100 along (0, 0.447, 0.894)", i.e. bbox
  delta (0, 44.72, 89.44), not (0, 50, 100). The hidden Movement readback stayed (0, 0, 100)
  while Direction was X — the Movement row is only the Custom-direction storage.
- **Bevel Shape options are Round, Solid, Curve, Step** — no Chamfer and no Concave, so the
  form's `bevel-chamfer` was run as Step/1 segment and `bevel-concave` as the third option
  (Solid); see the case rows.
- The Bevel "Segments" field is an integer (a fractional value set through the scripting API
  was rejected); Segments min 1, max 100.
- "Both Bevels" is the default section; only "Separate Bevel Controls" on unhides the
  "End Bevel" copy (ids differ; not exercised — no case in this form).
- "Bevel Outside" on pushes the bevel rings outward past the profile bounds (bbox ±210 with
  Size 10 on a 400-wide profile); face counts match the same bevel with the option off.
- Caps Density (0.2) and Quad Dominant are greyed at the default Caps Type; the values were
  read from the description, not while visible in the UI.
- The R25 Cylinder used by `tri-slice-orient-x` has the updated R25 defaults already noted in
  [r25-drift.md](r25-drift.md) §1 (Height Segments 4, Rotation Segments 16, Fillet Segments 3,
  Fillet Radius 3): 87 verts / 106 faces for the sliced +X case.
- Sweep defaults read back with Two Rail, Use Rail Direction, Use Rail Scale and Banking on
  (single path still works) — consistent with the [generators-3.md](generators-3.md) sweep rows.
- Probes: `~/forge-observer/probes/o56_disc*.py` (tab dumps), `o56_run1..5b.py` (cases, built on
  the validated O20 generator library); not committed per the cleanroom rules.