# Modeling tools II — observation (O25)

Status: partial (16 of 24 cases observed) · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: exactly as [modeling-tools.md](modeling-tools.md) (O6):
- A default cube made editable, with the segments given.
- The tool's **Reset Values** before each case.
- Selection by position, recorded as indices or point pairs.
- Export as `tests/golden/modeling-tools/<case>/mesh.obj`, with
  `observation = "docs/parity/modeling-tools-2.md"`.

Fill a settings table for each new tool, as O6 does. Commands with no reachable settings
get a "TODO: not readable" line.

Unblocks: batch 11 (U4 modeling tools).

How it was run: the cube was made with its Create-menu command, its segments set, then Make Editable. Selections were made by position (point numbers are the cube's own order: point 0 = (−100, −100, −100), 1 = (−100, 100, −100), 3 = (100, 100, −100), 5 = (100, 100, 100), 7 = (−100, 100, 100)). Extrude, Extrude Inner and Bevel ran through the tool's modeling command with the tool's Reset Values and only the named values changed (the full settings are in each `case.toml`). The other commands ran through the modeling command call with the settings named in the table, or (Split, Center Axis to) through their menu command on the selected object. Every other setting of those commands is **TODO: not readable**, since they have no settings container the script could reach.

## Settings per tool

Only the settings the cases set are listed; defaults of the rest of each command's dialog are **TODO: not readable**.

| Command | Set by the probe | Not read |
|---|---|---|
| Weld | nothing (defaults); the two points joined at their centre, (−100, 0, −100) | all defaults |
| Melt | nothing | all defaults |
| Split | nothing (menu command) | all defaults |
| Disconnect | Preserve Groups on | other defaults |
| Mirror | plane point (0, 0, 0), plane normal (1, 0, 0), Duplicate off | other defaults (Weld, Tolerance, Snap Points) |
| Connect Points/Edges | nothing | all defaults |
| Subdivide | Subdivisions 1 or 2, Hyper on or off | Angle, Pattern and the other defaults |
| Untriangulate | nothing | all defaults (angle, N-gons, respect options) |
| Center Axis to | nothing (menu command) | all defaults |
| Extrude / Extrude Inner / Bevel | as in [modeling-tools.md](modeling-tools.md) tables | – |

## Cases

| Case | Cube segments | Selection | Tool + settings | `golden info` | Notes |
|---|---|---|---|---|---|
| loopcut-mid | 1/1/1 | none | Loop/Path Cut across the 4 side faces at half height | – | **Not done**: see Notes |
| loopcut-quarter | 1/1/1 | none | the same, at a quarter of the height | – | **Not done**: see Notes |
| knife-line | 1/1/1 | top polygon | Knife in Line mode | – | **Not done**: see Notes |
| bridge-two | 1/1/1, top and bottom polygons deleted | the two open border loops | Bridge | – | **Not done**: see Notes |
| weld-two-points | 1/1/1 | points 0 and 1 | Weld (to their centre) | vertices 7, faces 6 (3-gons 2, 4-gons 4), bbox ±100, normals 6, uvs 6 | The joined point sits at (−100, 0, −100), the centre of the two |
| stitch | 2/1/1, Disconnect polygon 6 | – | Stitch and Sew | – | **Not done**: see Notes |
| melt-two | 2/1/1 | the 2 top polygons: 6, 7 | Melt | vertices 12, faces 9 (4-gons 8, 6-gons 1), bbox ±100, normals 6, uvs 6 | The two top polygons became one 6-gon; the point count stayed 12 |
| dissolve-edge | 2/1/1 | the middle top edge | Dissolve | – | **Not done**: see Notes |
| split-top | 1/1/1 | top polygon: polygon 4 | Split | vertices 12, faces 7 (4-gons 7), bbox ±100, normals 6, uvs 4 | The original keeps all 6 polygons; a second object `Cube.1` holds a copy of the selected polygon (4 points). The file holds both objects |
| disconnect-top | 1/1/1 | top polygon: polygon 4 | Disconnect, Preserve Groups on | vertices 12, faces 6 (4-gons 6), bbox ±100, normals 6, uvs 4 | |
| mirror-x | 1/1/1 | all polygons: 0–5 | Mirror, plane x = 0, no duplicate | vertices 8, faces 6 (4-gons 6), bbox ±100, normals 6, uvs 4 | A symmetric cube maps onto itself; only the point and corner order of each polygon is reversed, so the file's winding flips. A test on an asymmetric mesh showed the points do move across the plane |
| close-hole | 1/1/1, top polygon deleted | – | Close Polygon Hole | – | **Not done**: see Notes |
| connect-points | 1/1/1 | points 0 and 3 | Connect Points/Edges | vertices 8, faces 7 (3-gons 2, 4-gons 5), bbox ±100, normals 6, uvs 4 | The face holding both points is cut along the diagonal into two triangles |
| subdivide-smooth | 1/1/1 | all: 0–5 | Subdivide, 1 subdivision, Hyper on | vertices 26, faces 24 (4-gons 24), bbox ±100, normals 24, uvs 9 | Same topology as subdivide-1 (O6) but 24 normals, and the corners are not pulled in (bbox ±100): the exported mesh is not rounded |
| subdivide-2 | 1/1/1 | all: 0–5 | Subdivide, 2 subdivisions, Hyper off | vertices 98, faces 96 (4-gons 96), bbox ±100, normals 6, uvs 25 | |
| smooth | 4/4/4 | all | the Smooth tool or command | – | **Not done**: see Notes |
| untriangulate | 2/2/2, Triangulate first (26 points, 48 triangles) | all: 0–23 | Untriangulate, defaults | vertices 26, faces 24 (4-gons 24), bbox ±100, normals 6, uvs 9 | Back to the original 24 quads |
| center-axis | 1/1/1, all points moved by (50, 0, 0) | the object | Center Axis to, defaults | vertices 8, faces 6 (4-gons 6), bbox (−50, −100, −100)–(150, 100, 100), normals 6, uvs 4 | After: the object's position is (50, 0, 0) and its first point is back at (−100, −100, −100) in object space. The file carries the position, so the bounding box is shifted by 50 |
| array-command | 1/1/1 | all | the Array command, 3 copies, offset (300, 0, 0) | – | **Not done**: see Notes |
| extrude-ngon | 2/1/1 | the 2 top polygons: 6, 7 | Extrude, offset 50, Create N-gons on | vertices 18, faces 16 (4-gons 16), bbox (−100, −100, −100)–(100, 150, 100), normals 6, uvs 6 | Same counts as extrude-two (O6): the two top polygons share their inner edge, so no N-gon came out of the extrusion |
| inset-ngon | 2/1/1 | the 2 top polygons: 6, 7 | Extrude Inner, offset 20, Create N-gons on | vertices 18, faces 16 (4-gons 16), bbox ±100, normals 6, uvs 12 | Same counts as inset-two (O6) |
| bevel-solid | 1/1/1 | top-front edge: points 1 and 3 | Bevel, Bevel Mode Solid, offset 20 | vertices 12, faces 8 (4-gons 6, 6-gons 2), bbox ±100, normals 12, uvs 10 | Two more points than the chamfer (bevel-edge, 10 vertices): the bevel's end faces are 6-gons |
| bevel-proportional | 1/1/1 | top-front edge: points 1 and 3 | Bevel, Offset Mode Proportional, offset 30 % | vertices 10, faces 7 (4-gons 5, 5-gons 2), bbox ±100, normals 7, uvs 12 | The tool's container holds the offset as a fraction: 0.3 gave a distance of 60 along the 200-long edge (the new points at z = −40 and y = 40). A first run with the number 30 produced a degenerate mesh and was replaced |
| bevel-mitering-uniform | 1/1/1 | the top 4 edges: point pairs (1,7), (1,3), (3,5), (5,7) | Bevel, offset 20, subdivision 2, Mitering Uniform | vertices 20, faces 18 (4-gons 18), bbox ±100, normals 21, uvs 24 | |

## Notes

- **Cases not done (8): the scripted calls do nothing.** Each was tried with several settings combinations, through the modeling command call and, where it exists, the menu command with the object active and the right component mode. No error is raised; the mesh stays unchanged or the call returns false. Nothing is written for them (no case folder).
  - loopcut-mid, loopcut-quarter: the Loop/Path Cut command returned false in all modes, with edge, loop mode and offset 0.5 set. The Knife command (loop mode) returned true but changed nothing.
  - knife-line: the Knife's line mode needs a viewport line; not driven by script.
  - bridge-two: the Bridge command returned false with the two border edges given.
  - stitch: not tried in depth, since Bridge-type commands with edge or point pairs returned false; **TODO**: a GUI run.
  - dissolve-edge: the Dissolve command (menu command, selected edge, edge mode) returned without changing the mesh (12 points, 10 polygons before and after).
  - close-hole: the Close Polygon Hole command returned false with the hole's edge given.
  - smooth: the item called Smooth is a sculpt brush (a tool, not a mesh command); not driven by script.
- **array-command: dropped.** The Array command ran (24 points, 18 polygons) but its copies came out scaled and rotated (e.g. the second copy's x range 51–249, y ±91), and the third's bounding box reached x = 385, not 500. The settings the probe set are not the ones the form needs, and the container's fields were not matched to the dialog labels, so the case cannot be written down faithfully. **TODO**: map the Array command's fields (copies, offset, scale, rotation) from the GUI, then re-run.
- Mirror needs a plane point and normal in its settings to move anything; with the plane-number setting alone the points did not move.
- `case.toml` files hold only the settings the probe set for the commands without a readable container; Extrude, Extrude Inner and Bevel cases hold their full settings after Reset Values, as in O6.
