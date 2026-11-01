# Tags II — observation (O54)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted. Output in `tests/golden/tags-2/<case>/`, as a mesh or samples as the row
says, with `feature = "tags-2"`. Unblocks: B/027, H/106, 110.

Record each tag's parameter table once.

## Cases

| Case | Setup | Output |
|---|---|---|
| point-selection | a cube (segments 2/2/2) made editable; points 0–3 selected and stored with Set Selection | `mesh.obj` and `selection.json` (the stored indices) |
| polygon-selection | the same, polygons 0–1 stored | the same |
| edge-selection | the same, edge (0, 1) stored | the same |
| vertex-map-tag | the same cube; a vertex map tag with weights 0.0 … 1.0 set per point by script (weight = point index / point count) | `mesh.obj`; `samples.json` (kind `track`, param `weight`, frame = point index) |
| look-at-camera | a cube at (200, 0, 0) with a Look at Camera tag; the active camera at (0, 0, −600) | the cube's global rotation as clones samples (one entry) |
| protection | a cube with a Protection tag; then a scripted attempt to set its position to (100, 0, 0) | its position afterwards, as clones samples |
| display-tag | a cube with a Display tag, visibility 50 % | `mesh.obj` (unchanged geometry; this records that the tag doesn't change export) |
| compositing | a cube with a Compositing tag, "seen by camera" off | `mesh.obj`; note whether the OBJ export includes the cube |
| user-data | a Null with user data: a Float "amount" (default 1, range 0–10) and a Link "target" | `mesh.obj` of an empty scene plus a cube; record the user data descriptors in Notes |

## Notes
TODO
