# Tags II — observation (O54)

Status: observed (9 of 9 cases) · App version: Cinema 4D 2025.1.0 (25.010) · OS: macOS 26.7.2 (25G332, arm64) · Observer: Hermes · Date: 2026-11-20
Method: scripted. Output in `tests/golden/tags-2/<case>/`, as a mesh or samples as the row
says, with `feature = "tags-2"`. Unblocks: B/027, H/106, 110.

Record each tag's parameter table once.

The four tag tables, as the Attribute Manager shows them:

- **Point Selection** (the same tag serves polygons and edges): Priority, Enable; then a
  read-only Count and four buttons — Select, Deselect, Hide, Show.
- **Display**: Use Shading Mode, Shading Mode (Gouraud Shading / Gouraud Shading (Lines) /
  Quick Shading / Quick Shading (Lines) / Constant Shading / Constant Shading (Lines)),
  Style (Wireframe / Isoparms / Box / Skeleton), Volume Vector (Direction / Length),
  Use Level of Detail with Level of Detail 1.0, Use Visiblity with Visibility 1.0,
  Use Backface Culling.
- **Compositing**: Cast Shadows, Receive Shadows, Self Shadowing (all on), Seen by Camera,
  Seen by Rays, Seen by GI (all on), Compositing Background, Compositing Background for HDR
  Maps, Seen by Transparency (on), and the rest of its visibility block.
- **Protection**: Position (None / Lock / Limit), then X, Y, Z each with its own tick and a
  Min 0.0 / Max 0.0 pair; the ticks are on by default.
- **Look at Camera**: Priority, Enable, Change Pitch Rotation (on).
- **Vertex Map**: Priority, Enable, Color.

## Cases

| Case | Setup | Output | Result |
|---|---|---|---|
| point-selection | a cube (segments 2/2/2) made editable; points 0–3 selected and stored with Set Selection | `mesh.obj` and `selection.json` (the stored indices) | 26 points / 24 polygons; indices 0, 1, 2, 3 stored and read back (`selection.json` kind `point`) |
| polygon-selection | the same, polygons 0–1 stored | the same | indices 0, 1 stored and read back (kind `polygon`) |
| edge-selection | the same, edge (0, 1) stored | the same | edges 0, 1 stored and read back (kind `edge`) |
| vertex-map-tag | the same cube; a vertex map tag with weights 0.0 … 1.0 set per point by script (weight = point index / point count) | `mesh.obj`; `samples.json` (kind `track`, param `weight`) | 26 points; the tag takes a value per point index — weights read back 0, 0.038462, 0.076923, 0.115385, 0.153846, 0.192308, 0.230769, 0.269231 … i.e. index / 26 (recorded in `case.toml` as `first_weights`) |
| look-at-camera | a cube at (200, 0, 0) with a Look at Camera tag; the active camera at (0, 0, −600) | the cube's global rotation as clones samples (one entry) | the tag turns the cube to rotation (161.565051, 0, 0) deg — with Change Pitch Rotation on, the pitch takes the whole angle; the camera stays at (0, 0, −600) |
| protection | a cube with a Protection tag; then a scripted attempt to set its position to (100, 0, 0) | its position afterwards, as clones samples | the cube stays at (0, 0, 0): the scripted move is accepted without an error and silently ignored, so Position Lock is enforced against script writes |
| display-tag | a cube with a Display tag, visibility 50 % | `mesh.obj` (unchanged geometry; this records that the tag doesn't change export) | Visibility 0.5 recorded; the export is the plain 26-point cube — the tag changes nothing in `mesh.obj` |
| compositing | a cube with a Compositing tag, "seen by camera" off | `mesh.obj`; note whether the OBJ export includes the cube | Seen by Camera reads 0 (off) and the cube **is** still in the export: 26 `v` lines |
| user-data | a Null with user data: a Float "amount" (default 1, range 0–10) and a Link "target" | `mesh.obj` of an empty scene plus a cube; record the user data descriptors in Notes | both fields are created and take values: Float "amount" default 1.0, minimum 0.0, maximum 10.0, value 1.0; Link "target" holds the scene's Cube. The descriptors are recorded in `case.toml` under `[params.user_data]` |

## Notes

- The selections are stored through the object's own point/polygon/edge selection and then
  filed with the app's Set Selection command; the selection tag that appears carries the
  indices, and `selection.json` holds them as `{"kind": …, "indices": [...]}` — 0–3 for points,
  0–1 for polygons, 0–1 for edges. Reading the selection back through the object returns the
  same set.
- A vertex map tag accepts a value per point index directly (0 … 25 for this cube), which is
  how the 0 … 1 ramp was written; the first eight values are in `case.toml`.
- The Look at Camera tag produced a single pitch value (161.565051°) rather than a heading
  plus pitch: with the cube on the +X axis and the camera at −Z, the tag chose to rotate about
  X. Any implementation must reproduce that axis choice, not just "look at the camera".
- Protection blocks a scripted position write: the write does not raise, and the position
  afterwards is unchanged. The tag's X, Y and Z locks are on by default.
- The Display tag changes only the viewport representation (shading mode, style, level of
  detail, visibility), and the Compositing tag changes only render-time visibility: the OBJ
  export in both cases is the untouched cube, and "Seen by Camera off" does not remove the
  object from `mesh.obj`.
- User data is scriptable: a Float field takes default/minimum/maximum from its description
  container and a Link field is created from the base-list-link datatype with its default left
  empty; both accept values afterwards.
