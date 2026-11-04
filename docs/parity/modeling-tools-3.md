# Modeling tools III: follow-ups from briefs 020–024 and 031 — observation (O58)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: exactly as [modeling-tools-2.md](modeling-tools-2.md) (O25): a default cube made
editable with the segments given, the tool's **Reset Values** before each case, selection by
position (record the indices or point pairs), only the named values changed. Export as
`tests/golden/modeling-tools/<case>/mesh.obj` with
`observation = "docs/parity/modeling-tools-3.md"` and every tool setting in `case.toml`.

Why: briefs 020 (Extrude, Extrude Inner) and 021 (Bevel) implemented these rules from
flat, right-angled cube data only. Each case below settles one **parity-pending** rule
(see `docs/specs/MODELING.md` and the two briefs' Reports).

Unblocks: confirms 020/021; no brief waits on it.

## Cases

| Case | Cube segments | Selection | Tool and changes |
|---|---|---|---|
| extrude-bent-group | 2/2/2 | the two top polygons at +x, plus the two +x side polygons next to them (an L-shaped, non-flat group) | Extrude, offset 50, Preserve Groups on |
| extrude-bent-ngon | 2/2/2 | the same | Extrude, offset 50, Create N-gons on |
| extrude-max-angle | 2/2/2 | the same | Extrude, offset 50, Maximum Angle 10° |
| extrude-subdiv-3 | 1/1/1 | the top polygon | Extrude, offset 60, Subdivision 3 |
| extrude-hole | 3/1/3 | the 8 top polygons around the centre one (a ring with a hole) | Extrude, offset 40, Preserve Groups on |
| inset-patch | 3/1/3 | all 9 top polygons | Extrude Inner, offset 20 |
| inset-patch-sub | 3/1/3 | all 9 top polygons | Extrude Inner, offset 20, Subdivision 2 |
| extrude-bevel-sub | 1/1/1 | the top polygon | Extrude, offset 50, Subdivision 2, Bevel on (if the option exists; else skip) |
| bevel-wedge | a default Pyramid made editable | one base edge | Bevel, offset 20, Subdivision 2 |
| bevel-proportional-rect | segments 1/1/1 on a cube sized 200/100/300 | the top front edge | Bevel, offset mode Proportional, offset 0.3 |
| bevel-default-sub2 | 1/1/1 | the 4 top edges | Bevel, offset 20, Subdivision 2, Mitering Default |
| bevel-uniform-wedge | default Pyramid editable | two base edges meeting at a corner | Bevel, offset 20, Subdivision 2, Mitering Uniform |
| bevel-sub3 | 1/1/1 | the 4 top edges | Bevel, offset 20, Subdivision 3 |
| bevel-solid-sub1 | 1/1/1 | one top edge | Bevel, Shape Solid (or the solid option), offset 20, Subdivision 1 |
| bevel-limit-off | 1/1/1 | one top edge | Bevel, offset 150, Limit off |
| bevel-poly-multi | 2/2/2 | the 4 top polygons | Bevel in polygon mode, offset 20, extrusion 10 |
| bevel-point | 1/1/1 | point 3 (100, 100, −100) | Bevel in point mode, offset 20 |
| melt-patch | 3/1/3 | all 9 top polygons | Melt |
| melt-ring | 3/1/3 | the 8 top polygons around the centre one | Melt |
| melt-single | 1/1/1 | the top polygon | Melt |
| dissolve-edge | 2/2/2 | the top edge between the two +x top polygons | Dissolve |
| dissolve-point | 2/2/2 | the top centre point (0, 100, 0) | Dissolve |
| dissolve-poly | 2/2/2 | one top polygon | Dissolve |
| disconnect-groups-on | 2/1/1 | both top polygons | Disconnect, Preserve Groups on |
| disconnect-groups-off | 2/1/1 | both top polygons | Disconnect, Preserve Groups off |
| disconnect-all | 1/1/1 | all polygons | Disconnect |
| optimize-two-cubes | two default cubes made editable and joined with Connect (Weld off), the second moved to x = 200 | – | Optimize, tolerance 0.01 |
| mirror-duplicate | 1/1/1 cube moved to x = 50 | all polygons | Mirror, plane x = 0, Duplicate on, Weld on |
| mirror-subset | 1/1/1 | the top polygon | Mirror, plane x = 0 |
| subdivide-3 | 1/1/1 | – | Subdivide, Subdivisions 3, Hyper off |
| subdivide-tri | a default Pyramid made editable | – | Subdivide, Subdivisions 1 |
| subdivide-subset | 1/1/1 | the top polygon | Subdivide, Subdivisions 1 |
| triangulate-ngon | a default Disc (Disc segments 1, rotation segments 6) made editable | – | Triangulate |
| untriangulate-ngons | 1/1/1 triangulated | – | Untriangulate, N-gons on (record the angle) |
| sel/grow-poly | 4/4/4 | one middle front polygon | Grow Selection (defaults; record its settings). Output `selection.json` |
| sel/grow-point | 4/4/4 | one middle front point (point mode) | Grow Selection. Output `selection.json` |
| sel/grow-edge | 4/4/4 | one middle front edge (edge mode) | Grow Selection. Output `selection.json` |
| sel/convert-tolerant-p2poly | 2/2/2 | the start of O26 convert-point-to-poly | Convert Selection points → polygons, Tolerant on. Output `selection.json` and the edit mode afterwards |
| sel/convert-tolerant-e2poly | 2/2/2 | the start of O26 convert-edge-to-poly | Convert Selection edges → polygons, Tolerant on. Output `selection.json` |
| sel/connected-point-only | two default cubes made editable, Connect (Weld off), then Optimize so they share only one corner point (second cube at (200, 200, 0)) | one polygon of the first cube | Select Connected. Output `selection.json` |
| knife-plane-mid | 1/1/1 | all polygons | Knife in Plane mode, plane through (0, 0, 0) with normal (0, 1, 0), if it can be set by script (else write "gui-only" and skip) |
| slide-loop | 2/2/2 | the 4 middle edges around the cube at y = 0 (an edge loop) | Slide, offset 0.5 toward +y (if scriptable; record the settings and their range; else "gui-only") |
| slide-edge | 2/2/2 | one middle edge at y = 0 on the front face | Slide, offset 0.5 (if scriptable) |
| arr-f/array-3 | follow-up from brief 031: 1/1/1 cube made editable | – | Array command with 3 copies (record every dialog field label and default); export, and note whether the result has 3 or 4 cubes |
| arr-f/array-mode | 1/1/1 | the top polygon | Array command, 2 copies, offset (0, 0, 300), each mode the dialog offers; note whether only the selection is copied |
| arr-f/duplicate | default cube (not editable) | – | Duplicate, 3 copies, offset (200, 0, 0) per step if offered; record the copy names and positions (`scene_snapshot` in Notes) |
| arr-f/arrange | three default cubes | – | Arrange (each layout it offers); record the resulting positions in Notes |
| arr-f/randomize | three default cubes | – | Randomize with seed 1 and ranges P 100, R 30°, S 0.5; record the resulting transforms in Notes |
| arr-f/transfer | two cubes, the second moved to (200, 100, 0) and rotated H 45° | – | Transfer from the second to the first; record what changed (position, rotation, scale) and in which space |

GUI read (add to G3): the default of every setting in the Subdivide, Optimize, Weld, Melt,
Dissolve, Disconnect, Mirror and Triangulate dialogs.

GUI-only: the drag scale (how many units the Extrude offset changes per screen pixel at the
default camera). Add it to [GUI-SESSIONS.md](GUI-SESSIONS.md) G3 when someone sits at the app.

## Notes
- `bevel-proportional-rect` crashed the app in the run of 2026-11-03/04 (leftover
  CURRENT_CASE said so at the start of the next run). Skipped for good, per the queue
  rules.
