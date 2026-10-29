# Re-checks from O1 and O4 — observation (O5)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: scripted where possible ([agents/OBSERVER.md](../agents/OBSERVER.md)). Unblocks:
three planner rulings in brief 07 that rest on a single reading.

## 1. Slice "To" default

Make a cylinder, cone, disc, torus, tube and capsule with their menu commands. For each,
read Slice > To back through the scripting interface **and** through `get_parameter`, and
record both in degrees. If the two disagree, say so.

The two readings agree for all six. Both return 3.141592653589793 (radians), which is 180°.
Slice (off by default) and From (0°) were read in the same run.

| Object | To (probe) | To (get_parameter) |
|---|---|---|
| cylinder | 180° | 180° |
| cone | 180° | 180° |
| disc | 180° | 180° |
| torus | 180° | 180° |
| tube | 180° | 180° |
| capsule | 180° | 180° |

## 2. Cap type of Lathe, Loft and Sweep

O4 reads N-gon caps for all three, while the R20 meshes show triangles. Settle it with meshes:

| Case | Scene (menu commands) | Export to | Cap faces seen (count, corners per face) |
|---|---|---|---|
| lathe-defaults | Lathe with a default Rectangle spline as its child | `tests/golden/rechecks/lathe-defaults/mesh.obj` | none. 128 vertices, 128 faces, all 4-gons (32 subdivisions × 4 sides), so the 360° revolution is closed. Caps tab: Start Cap on, End Cap on, Caps Type **N-gon** |
| loft-defaults | Loft with two default Circle splines as children, the second at position y 200 | `tests/golden/rechecks/loft-defaults/mesh.obj` | **2 cap faces, 16 corners each** (N-gons), one per end. The other 64 faces are quads (16 around × 4 rows). Caps Type reads **N-gon** |

`golden info`:

- lathe-defaults: vertices 128, faces 128 (4-gons 128), bbox ±200 on all axes, normals 35, uvs 165.
- loft-defaults: vertices 80, faces 66 (4-gons 64, 16-gons 2), bbox (-200, -200, 0) to (200, 400, 0), size (400, 600, 0), normals 2, uvs 98.

Lathe has no caps on a closed revolution: the export has no cap faces. The Caps tab
default is Caps Type N-gon (Start Cap and End Cap on). Loft's mesh confirms the N-gon default.

Sweep was not in the case list and was not exported, so its cap type is still unconfirmed from a mesh.

The R25 default Rectangle and Circle exported at ±200 (not ±100), so the lathe and the
loft are 400 wide.

## 3. Boole default type

Read Boolean Type after creating the Boole object through its menu command in each of
three ways: nothing selected, one cube selected, and two cubes selected.

| How created | Boolean Type read | Children it got |
|---|---|---|
| nothing selected | A subtract B (value 1) | none |
| one cube selected | A subtract B (value 1) | none; the cube stayed at the top level |
| two cubes selected | A subtract B (value 1) | none; both cubes stayed at the top level |

The Boolean Type options in order are A union B, A subtract B, A intersect B, A without B.

## Notes

- **Boole default type is A subtract B.** [r25-drift.md](r25-drift.md) records the R25 default as union (R20: A subtract B). A Boole made through its menu command reads A subtract B in all three ways. The drift reading was probably taken from an object not made by the menu command. Not checked which; the planner decides.
- The menu command did not move the selected cubes under the Boole when run from the script. This does not show what a click in the GUI does, which is covered by the gui-only O10.
- Both meshes were exported with the OBJ settings in [r25-drift.md](r25-drift.md) §1.
- The loft file's cap faces carry some out-of-sequence texture and normal indices (one reads `11/95/0`, with normal index 0). `golden check` was still run on it, see the commit.
