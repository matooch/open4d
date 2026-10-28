# Make Editable and Current State to Object — observation (O3)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-28
Method: scripted or GUI ([agents/OBSERVER.md](../agents/OBSERVER.md)); level A for the
meshes, level C for the tree. Unblocks: the parity-pending rules of brief 03.

For each case, record the **object tree after the command** (names, types as shown,
order, tags, children) and export the result as OBJ with the usual settings to
`tests/golden/make-editable/<case>/mesh.obj` (for a tree with several polygon objects,
export the whole scene and note which object is which group).

## Cases

| Case | Scene before | Command | Tree after (names / types / tags / children) | Notes |
|---|---|---|---|---|
| cube | default cube with its smoothing tag | Make Editable | `Cube` (type Polygon, 8 points, 6 polygons); tags in order: Phong, UVW, then two unnamed tags (the point and polygon data the editable object stores; not shown in the Objects list). No children. | name kept; Phong kept; a UVW tag is added; **no new selection tags**. |
| cube-deformed | cube (segments 4/8/4, size_y 400) > Bend strength 90 | Make Editable on the cube | `Cube` (Polygon, 162 points, 160 polygons; tags Phong, UVW, two unnamed data tags) > child `Bend` (Bend, enabled, strength still 90°). | the bend is **not applied**: the points are the unbent box (bbox ±100 × ±200 × ±100); the Bend stays as a child and still deforms live, so the exported mesh is bent (bbox -100…359.15, -200…259.15). The Bend was inserted under the cube by the probe: the Create menu with the cube selected makes it a separate top-level object. |
| cube-deformed-csto | same | Current State to Object on the cube | Top level, in order: `Cube` (Cube, unchanged, tag Phong) > child `Bend`; then **`Cube`** (Polygon, 162 points, 160 polygons, tags Phong, UVW, two data tags), no children. | the new object is the **next sibling after the original**, with the **same name** (no suffix; the OBJ exporter names them Cube and Cube_1). Its points are the bent result (bbox -100…359.15, -200…259.15). The original keeps its Bend child and stays selected. The export has both, 320 faces. |
| subdivision | Subdivision Surface > default cube | Make Editable on the generator | One top-level object `Subdivision Surface` (Polygon, 98 points, 96 polygons; tags Phong, UVW, two data tags). No children. | **one object**, named after the generator; the input cube is gone (consumed); the Phong tag came from the cube. Geometry = the subdivided cube (level 2). The cube was placed under the generator by the probe. |
| subdivision-csto | same | Current State to Object | | |
| extrude | Extrude > Rectangle spline (defaults) | Make Editable | `Extrude` (Polygon, 8 points, 4 polygons: the side walls; tags Phong, UVW, two data tags) with two children in order: `Cap 1` (Polygon, 4 points, 1 polygon; tags UVW, Polygon Selection named C1, Phong, two data tags) and `Cap 2` (same, selection tag C2). | **caps are separate polygon objects, as children** of the extruded object, named Cap 1 (the start cap) and Cap 2. The Rectangle input is gone. In the OBJ: groups Extrude (4 faces), Cap_1 (1), Cap_2 (1); bbox z 0…100. |
| array | Array > default cube | Make Editable | | |
| sphere-moved | sphere at position (10, 20, 30), rotation H 45° | Make Editable | `Sphere` (Polygon, 114 points, 128 polygons; tags Phong, UVW, two data tags), position (10, 20, 30), rotation H 45° unchanged. | position and rotation stay **on the object**. Name kept; the exported bbox is centred at (10, 20, 30). |
| spline | Circle spline | Make Editable | `Circle` (type Spline, 4 points, closed; two unnamed data tags: segment data and point data). No children. | name kept; becomes an editable spline with 4 points. Tree only, no export. |

## Behaviour notes
- **Undo:** each command (Make Editable, Current State to Object) is **one undo step**: a single Undo restored the exact tree before the command in all 9 cases (checked with the scripting interface's undo, not the menu).
- **Selection after:** the object the command was run on stays the only selected object, in every case. For Make Editable it is the converted object (now polygon, Null or spline); for Current State to Object it is the **original**, not the new copy. The caps of the extrude are not selected.
- Both commands were run through their command (by command id) with the object selected; menu paths and shortcut keys were not read.
- Creating an object from the Create menu while another is selected makes a **separate top-level object**, never a child; the child relations (cube under Subdivision Surface, Array, Extrude; Bend under cube) were set by the probe, as dragging in the Objects list would.
- Every Make Editable result carries two unnamed data tags after Phong and UVW (point and polygon data; segment and point data for the spline). They are not selection tags: **no new selection tags** appear except Cap 1 / Cap 2's `C1`/`C2` polygon selections after the Extrude conversion.
- Exports: whole scene, the usual settings (scale 1 cm, normals Vertices, UVs Original, no material, Flip Z off); R25 writes six decimals and an `o` line per object. Which object is which: cube-deformed-csto `Cube` = original (bent live), `Cube_1` = new copy; subdivision-csto `Cube` = the input under the generator (subdivided live), `Subdivision_Surface` = new copy; extrude `Extrude`, `Cap_1`, `Cap_2`; array `Cube.8` … `Cube.1`. For the CSTO cases the file holds both objects, so counts double (324 points / 320 faces; 196 / 192).
- Not read: the menu texts' exact labels, keyboard shortcuts, any dialog the commands may show, behaviour with several objects selected.
