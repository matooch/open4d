# Re-checks II — observation (O15)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)). Folder
`tests/golden/rechecks/<case>/`, `feature = "rechecks"`,
`observation = "docs/parity/rechecks-2.md"`.

## 1. Sweep caps on an open path

O11's sweep had no caps because its Rectangle path is closed. Use an open path instead:

| Case | Scene (menu commands) | Cap faces seen (count, corners) | Caps Type read |
|---|---|---|---|
| sweep-open | Sweep, child 1: default Circle (profile); child 2: a Helix with default values (path) | In the exported mesh: 32 400 quads (the sweep surface, 36 profile points × 900 path segments) and **68 triangles**, 34 per end. The 68 triangles use **72 extra vertices** (36 per end) that lie at the same positions as ring vertices already in the surface: the mesh has 32 508 vertices. The evaluated object itself lists only the 32 400 quads (32 436 points); the caps are not among its polygons | N-gon (option 2, the default) |
| sweep-open-triangles | The same, with Caps Type set to the triangle option | **68 triangles**, same count as above, but they use the surface's own vertices: the mesh has 32 436 vertices (no extra ones). The evaluated object lists 32 468 polygons: 32 400 quads and 68 triangles | Triangles (option 0 of: Triangles, Quadrangles, N-gon, Delaunay, Regular Grid) |

Helix defaults as read: Start Radius 200, Start Angle 0°, End Radius 200, End Angle 720°, Radial Bias 50 %, Height 200, Height Bias 50 %, Subdivision 100, Plane XY, Reverse off, Intermediate Points Uniform. Circle defaults: Radius 200, Plane XY, Intermediate Points Uniform. Start Cap and End Cap on the Sweep are on (their default). Both are in `case.toml`; the Sweep's own parameters are written in full, as in `rechecks/sweep-defaults`.

In the first case the caps are written to the mesh file as 68 triangles with their own vertices (two 36-sided faces, each cut into 34 triangles by the exporter); how the app stores them is not visible from the evaluated polygons. I did not look at the cap normals.

## 2. Extrude bevel needs subdivision?

| Case | Scene | Changed |
|---|---|---|
| extrude-bevel-subdiv | 1/1/1 cube made editable, top polygon | Extrude, offset 50, Bevel 10, Subdivision 2 |

Compare with `modeling-tools/extrude-subdiv` (Bevel 0): note any difference in vertex
count or bbox.

Result: **no difference.** 20 points and 18 polygons (all quads), bbox (−100, −100, −100) to (100, 150, 100); the exported vertices and faces are identical, in the same order, to `modeling-tools/extrude-subdiv`. Two things to know about that comparison: `extrude-subdiv`'s `case.toml` has `bevel = 5.0`, not 0 (the value the tool had after its reset; the form's "Bevel 0" is not what that case holds), and a third run with Bevel 0 and Subdivision 2 gave the same 20 points, 18 polygons and bbox (not saved). So with Subdivision 2 the Bevel value (0, 5 or 10) changes nothing in these measurements.

## Notes
- All three cases wrote a `mesh.obj` and a `case.toml`; nothing failed.
- Not checked: other Caps Type options (Quadrangles, Delaunay, Regular Grid); cap faces on the Helix with Start/End Cap off; extrude Bevel with Subdivision 1 or a different offset (the earlier `extrude-bevel` case is Subdivision at its default); the cap normals.
