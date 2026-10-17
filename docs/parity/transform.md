# Transforms — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-17
Unblocks: object transforms, rotate gizmo, object-local gizmo axes, coordinate panel.

All cases use a **default cube** (its vertex order is fixed, so per-vertex comparison
reveals the exact rotation math). Folder: `tests/golden/transform/<case>/`.
`[params]` keys: `pos_1..3`, `rot_1..3`, `scale_1..3` = the 1st/2nd/3rd field of each
group **in the order the app shows them** (we don't assume which axis they mean).
Export settings: [export-settings.md](export-settings.md) (Flip Z Axis off, No Material).

## The coordinate fields (as shown)

Two places show them. All values were typed into the **Attributes panel, "Coord." tab**
of the selected object (tabs there: Basic, Coord., Object, Phong). Its fields are laid
out in three columns: P.X / P.Y / P.Z, then S.X / S.Y / S.Z, then R.H / R.P / R.B, with
an "Order" dropdown under the rotation column and collapsed groups "Quaternion" and
"Freeze Transformation" below. The separate **Coordinates manager** (bottom of the
window) shows columns Position (X, Y, Z), Size (X, Y, Z), Rotation (H, P, B) and an
"Apply" button.

| Group | Field 1 label | Field 2 label | Field 3 label | Unit shown | Default |
|---|---|---|---|---|---|
| Position | P . X (Coord. manager: X) | P . Y (Y) | P . Z (Z) | cm | 0 cm, 0 cm, 0 cm |
| Rotation | R . H (Coord. manager: H) | R . P (P) | R . B (B) | ° | 0 °, 0 °, 0 ° |
| Scale | S . X | S . Y | S . Z | none (plain number) | 1, 1, 1 |
| Size (if separate) | Coord. manager "Size" column: X | Y | Z | cm | 200 cm, 200 cm, 200 cm (default cube); shows 400 / 600 / 800 cm after scale (2, 3, 4) |

- Rotation field 1 (R.H) turns the cube around the vertical axis: the green (Y)
  axis, the top face stays horizontal. Field 2 (R.P) turns it around the red (X)
  axis: the red gizmo arrow keeps its direction. Field 3 (R.B) turns it around the
  blue (Z) axis: the blue gizmo arrow keeps its direction. (Each seen alone, value 30,
  from a new scene.)
- Is there a rotation-order setting? Yes, "Order" dropdown in the Coord. tab.
  Options: HPB, XYZ, XZY, YXZ, YZX, ZXY, ZYX. Default: HPB (used for every case).
- Object vs world coordinates switch? In the Coordinates manager only: dropdown with
  Object (Rel), Object (Abs), World; default Object (Rel). Its second dropdown
  switches the middle column: Scale, Size, Size+; default Size. The Attributes Coord.
  tab has no such switch.

## Cases

All 8 cases: each from File > New, one default cube added with the toolbar Cube
button, values typed into the Attributes Coord. tab. Order = HPB throughout.

| Case | Set (field order) | `golden info` (vertices / faces / size) | Notes |
|---|---|---|---|
| identity | nothing | 8 / 6 (4-gons 6) / size (200, 200, 200), centre (0, 0, 0) | bbox (-100, -100, -100)..(100, 100, 100) |
| rot-1-30 | rot_1 = 30 (R.H) | 8 / 6 (4-gons 6) / size (273.205078125, 200, 273.205078125), centre (0, 0, 0) | |
| rot-2-30 | rot_2 = 30 (R.P) | 8 / 6 (4-gons 6) / size (200, 273.205078125, 273.205078125), centre (0, 0, 0) | |
| rot-3-30 | rot_3 = 30 (R.B) | 8 / 6 (4-gons 6) / size (273.205078125, 273.205078125, 200), centre (0, 0, 0) | |
| rot-combined | rot = (30, 45, 60) | 8 / 6 (4-gons 6) / size (281.4313049316406, 334.60650634765625, 303.9057922363281), centre (0, 0, 0) | the one that reveals rotation order |
| scale | scale = (2, 3, 4) | 8 / 6 (4-gons 6) / size (400, 600, 800), centre (0, 0, 0) | object scale S.X/S.Y/S.Z in the Attributes Coord. tab, not the cube's Size parameters. Export bakes scale. |
| full | pos = (10, 20, 30), rot = (30, 45, 60), scale = (2, 3, 4) | 8 / 6 (4-gons 6) / size (889.6393432617188, 1022.7664489746094, 878.1258544921875), centre (10, 20.000015258789063, 30) | |
| child | cube as child of a null at pos (0, 50, 0), rot_1 = 90; cube pos (100, 0, 0) | 8 / 6 (4-gons 6) / size (200, 200, 200), centre (0, 50, 100) | bbox (-100, -50, -0.00000000000001)..(100, 150, 200). See procedure below. |

Child procedure: new scene; cube added and P.X = 100 set; Null added (Create > Object >
Null) and given P.Y = 50, R.H = 90; then the Cube was dragged onto the Null in the
Objects list. **Dragging kept the cube's world placement**: its Coord. tab then showed
P = (0, -50, -100) cm, R.H = -90 °. The cube's values were then retyped to
P = (100, 0, 0), R = (0, 0, 0) before export; the Null still showed P = (0, 50, 0),
R.H = 90. Only the cube was selected at export (the Null is shown in a dimmer
highlight as parent of the selection). With the cube selected, the Coordinates manager
read: Object (Rel) and Object (Abs) = position (100, 0, 0), rotation (0, 0, 0);
World = position (0, 50, 100), rotation H 90 °, P 0 °, B 0 °.

## Behaviour notes
- Does the exported OBJ contain world-space points (transforms baked)? Yes. Position,
  rotation and object scale are baked into the `v` lines (identity file has ±100;
  scale case has ±200/±300/±400; full case centre = typed position), and the parent
  Null's transform is baked into the child cube (child centre (0, 50, 100) equals the
  World position shown in the Coordinates manager). Each file holds one object
  (`o Cube`); the Null writes nothing. Each file still has 6 `vn` lines and 4 `vt`.
- Anything that changes when you switch object/world coordinates: only the numbers the
  Coordinates manager displays (child cube: Object (Rel)/(Abs) show local values,
  World shows (0, 50, 100) and H 90 °). The Attributes Coord. tab keeps showing local
  values. The switch was set back to Object (Rel); it has no effect on export (all
  exports were made in Object (Rel)).
- Small float residue in output: the child file writes `-0.00000000000001` and
  `0.00000000000001` as the Z of the four vertices on the near face (where 0 would be
  expected); the full case centre Y is 20.000015258789063.
