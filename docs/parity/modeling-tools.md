# Modeling tools — observation (O6)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)): the tools run through the
app's documented modeling-command scripting call, or `run_modeling_command`. Level A
(exact meshes). Unblocks: batch 11 (U4 modeling tools).

## Setup for every case

- New document. Make a **default cube** with its menu command, set the segments the row
  gives, then Make Editable.
- Select the components the row names. Our cube has the reference's vertex and polygon
  order, so find them by position:
  - "top polygon" is the polygon whose points all have y = +100;
  - "top edges" are the 4 edges of that polygon;
  - "top-front edge" is the edge with y = +100 and z = −100.
  Record the indices you selected in the row.
- Run the tool with the settings the row gives. Every other setting stays at the tool's
  default.
- Export the cube with the usual OBJ settings to `tests/golden/modeling-tools/<case>/mesh.obj`.
- `case.toml`: `feature = "modeling-tools"`. `[params]` holds `tool`, the selection
  (`select_polys = [...]`, `select_edges = [[a, b], ...]` as point pairs,
  `select_points = [...]`), the cube's segment counts, and every tool setting you could
  read, under our key names (lowercase words with `_`).

How it was run: before each tool case the tool's own Reset Values button was pressed, so
every setting not named in the row is the tool's factory default. The case then ran the tool
through the modeling command call with that settings container and only the named values
changed. Selections were made by position as above. An edge was selected on both of its
sides (the edge index of each polygon that has it); the indices are in `select_edges` as
point pairs, the point numbers being the cube's point order.

## Tool settings (fill once per tool)

For each tool, list every setting as it is shown: label, default, and the min and max
that a ±999999999 assignment keeps, as in r25-defaults.md. Read them from the tool's
settings container (or from the GUI, if the script can't reach them).

**Min and Max are the limits the tool declares for the field, not what a ±999999999
assignment kept.** The tool's settings container is a plain container with no clamping, so
an assignment test through it would read back the typed value. The declared limits are
what the field enforces. Angles are shown in degrees (stored in radians), lengths in cm.
Defaults are the values after Reset Values.

### Extrude
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Offset | `offset` | 5 | −1e20 | 1e20 |
| Offset Variation | `offset_variation` | 0 | 0 | 1e18 |
| Subdivision | `subdivision` | 0 | 0 | 1e20 |
| Create N-gons | `create_ngons` | off | – | – |
| Preserve Groups | `preserve_groups` | on | – | – |
| Create Caps | `create_caps` | off | – | – |
| Maximum Angle | `maximum_angle_deg` | 91° | 0° | 180° |
| Rotation | `rotation_deg` | 0° | −1e20° (−1.745e18 rad) | 1e20° (1.745e18 rad) |
| Quantize Rotation | `quantize_rotation` | on | – | – |
| Angle (quantize) | `quantize_angle_deg` | 5° | 0° | 180° |
| Bevel | `bevel` | 5 | −1e20 | 1e20 |
| Bevel Variation | `bevel_variation` | 0 | 0 | 1e18 |
| Realtime Update | `realtime_update` | on | – | – |

The container also holds one value (default off) that has no entry in the tool's field list. Its meaning was not confirmed, so it is not in `case.toml`.

### Extrude Inner (Inset)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Offset | `offset` | 5 | −1e20 | 1e20 |
| Offset Variation | `offset_variation` | 0 | 0 | 1e18 |
| Subdivision | `subdivision` | 0 | 0 | 1e20 |
| Create N-gons | `create_ngons` | off | – | – |
| Preserve Groups | `preserve_groups` | on | – | – |
| Maximum Angle | `maximum_angle_deg` | 89° | 0° | 180° |
| Realtime Update | `realtime_update` | on | – | – |

### Bevel
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Bevel Mode | `bevel_mode` | Chamfer | – | options: Chamfer, Solid |
| Offset Mode | `offset_mode` | Fixed Distance | – | options: Fixed Distance, Radial, Proportional |
| Offset | `offset` | 10 | −1e20 | 1e20 |
| Subdivision | `subdivision` | 0 | 0 | 1e20 |
| Depth | `depth` | 1 | −1e18 | 1e18 |
| Limit | `limit` | on | – | – |
| Shape | `shape` | Round | – | options: Round, User, Profile |
| Tension | `tension` | 1 | −1e18 | 1e18 |
| Mitering | `mitering` | Default | – | options: Default, Uniform, Radial, Patch |
| Ending | `ending` | Default | – | options: Default, Extend, Inset |
| Partial Rounding | `partial_rounding` | None | – | options: None, Full, Convex |
| Corner N-gons | `corner_ngons` | off | – | – |
| Rounding N-gons | `rounding_ngons` | off | – | – |
| Phong Break Rounding | `phong_break_rounding` | off | – | – |
| Phong Break Miters | `phong_break_miters` | off | – | – |
| Realtime Update | `realtime_update` | on | – | – |

Fields the tool hides in the modes used here (values after Reset Values): a second Depth 10 (−1e20 to 1e20), Extrusion 10 (−1e20 to 1e20), Max Angle 89° (0° to 1e20°), Preserve Groups on, Symmetry on, Constant Cross Section on, Profile Plane XY (options XY, XZ, ZY), plus a user-shape curve and a profile-spline link.

### Commands without readable settings

Subdivide, Triangulate, Reverse Normals and Optimize have no tool settings container the
script could reach, and their dialogs were not opened (that is GUI work). What each case
ran with:

- Subdivide: Subdivisions 1 and Hyper (smoothing) off were set by the probe; every other setting is **TODO: not readable**.
- Triangulate, Reverse Normals: no settings passed. **TODO: defaults not read.**
- Optimize: an empty settings container was passed. **TODO: tolerance and the three option defaults not read.**

## Cases

| Case | Cube segments | Selection | Tool + settings | `golden info` | Notes |
|---|---|---|---|---|---|
| extrude-top | 1/1/1 | top polygon: polygon 4 | Extrude, offset 50 | vertices 12, faces 10 (4-gons 10), bbox (−100, −100, −100)–(100, 150, 100), normals 6, uvs 4 | |
| extrude-top-negative | 1/1/1 | top polygon: polygon 4 | Extrude, offset −50 | vertices 12, faces 10 (4-gons 10), bbox ±100, normals 6, uvs 4 | Extrudes inward: the top stays at y 100 in the bounding box |
| extrude-two | 2/1/1 | the 2 top polygons: 6, 7 | Extrude, offset 50, Preserve Groups **on** | vertices 18, faces 16 (4-gons 16), bbox (−100, −100, −100)–(100, 150, 100), normals 6, uvs 6 | |
| extrude-two-separate | 2/1/1 | the 2 top polygons: 6, 7 | Extrude, offset 50, Preserve Groups **off** | vertices 20, faces 18 (4-gons 18), bbox (−100, −100, −100)–(100, 150, 100), normals 6, uvs 6 | Two more vertices and faces than with groups preserved: each polygon is extruded alone |
| inset-top | 1/1/1 | top polygon: polygon 4 | Extrude Inner, offset 20 | vertices 12, faces 10 (4-gons 10), bbox ±100, normals 6, uvs 8 | |
| inset-two | 2/1/1 | the 2 top polygons: 6, 7 | Extrude Inner, offset 20, Preserve Groups on | vertices 18, faces 16 (4-gons 16), bbox ±100, normals 6, uvs 12 | |
| bevel-poly | 1/1/1 | top polygon: polygon 4 | Bevel (polygon mode), offset 20, subdivision 0 | vertices 12, faces 10 (4-gons 10), bbox (−100, −100, −100)–(100, 110, 100), normals 13, uvs 8 | The top moved up by 10, which equals the hidden Extrusion default (10); that link is an inference, not tested |
| bevel-edge | 1/1/1 | top-front edge: points 1 and 3 (edge indices 1, 19) | Bevel (edge mode), offset 20, subdivision 0 | vertices 10, faces 7 (4-gons 5, 5-gons 2), bbox ±100, normals 9, uvs 12 | The app's polygon object reports 9 polygons; the file has 7 faces because the exporter joins the two 5-sided faces |
| bevel-edge-sub | 1/1/1 | top-front edge: points 1 and 3 (edge indices 1, 19) | Bevel (edge mode), offset 20, subdivision 2 | vertices 14, faces 9 (4-gons 7, 7-gons 2), bbox ±100, normals 12, uvs 20 | The app reports 13 polygons (same n-gon joining) |
| bevel-edges-top | 1/1/1 | top edges: point pairs (1,7), (1,3), (3,5), (5,7) (edge indices 13, 16, 1, 19, 5, 18, 9, 17) | Bevel (edge mode), offset 20, subdivision 1 | vertices 16, faces 14 (4-gons 14), bbox ±100, normals 21, uvs 18 | |
| subdivide-1 | 1/1/1 | all polygons: 0–5 | the Subdivide command, 1 subdivision, smoothing off | vertices 26, faces 24 (4-gons 24), bbox ±100, normals 6, uvs 9 | |
| triangulate | 2/2/2 | all polygons: 0–23 | the Triangulate command | vertices 26, faces 48 (3-gons 48), bbox ±100, normals 13, uvs 9 | |
| reverse-normals | 1/1/1 | top polygon: polygon 4 | the Reverse Normals command | vertices 8, faces 6 (4-gons 6), bbox ±100, normals 5, uvs 4 | Topology unchanged; the normal count is 5 (a plain cube has 6) |
| optimize | 1/1/1 after Connect of two cubes at the same place | all | the Optimize command, defaults | vertices 8, faces 6 (4-gons 6), bbox ±100, normals 6, uvs 4 | See Notes: Connect had already welded the cubes, so Optimize had nothing to merge |

## Notes

- Bevel's command number differs from its tool number: the modeling command call fails (returns false, mesh unchanged) with the number the scripting constant gives and works with the tool's own number. The probe uses the working one. This stays in the observer workspace.
- Optimize is a weak case. A Connect of two cubes at the same place has Weld on with Tolerance 0.1 by default, so Make Editable gave 8 points and 6 polygons already. Optimize then changed nothing. To test Optimize on duplicate points, the planner can add a case with Connect's Weld off; not run, since the form does not list it.
- The Extrude container holds a Maximum Angle of 91° after Reset Values, while Inset and Bevel read 89°. Read as is; not cross-checked in the GUI.
- Modeling-command results were taken from the exporter's file. The numbers in the app's polygon object can be higher where n-gons exist (bevel-edge, bevel-edge-sub).
- Not done: Min and Max by assignment (see the note above the tables); the dialogs of Subdivide, Triangulate, Reverse Normals and Optimize.
