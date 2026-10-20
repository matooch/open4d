# Symmetry generator — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/symmetry/<case>/`.

## Where it lives
Create > Modeling > Symmetry (new scene each case: File > New). The default cube is
added first, moved in its Coord. tab, then dragged onto the Symmetry in the Objects list.
Attributes tabs: Basic, Coord., Object.

Input: one default Cube as the child of the generator.

**What the export contains:** exporting with the Symmetry (and child) in the scene
writes the generated result directly, as one object named after the child (`o Cube`);
no Current State to Object needed. `input.obj` was exported from the same scene with the
generator disabled (its green check in the Objects list switched off), which writes
only the child cube.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Mirror Plane | `mirror_plane` | enum | ZY | – | – | – | options XY, ZY, XZ |
| Weld Points | `weld_points` | bool | on | – | – | – | off greys out Tolerance and Symmetrical |
| Tolerance | `tolerance` | float | 0.01 | not tested | not tested | cm | |
| Symmetrical | `symmetrical` | bool | on | – | – | – | |
| Modeling > Clamp Points on Axis | `clamp_points_on_axis` | bool | off | – | – | – | modeling aid, not used |
| Modeling > Delete Polygons on Axis | `delete_polygons_on_axis` | bool | off (greyed) | – | – | – | not used |
| Modeling > Automatically Flip | `auto_flip` | bool | off | – | – | – | not used |
| Modeling > Flip | – | button | – | – | – | – | not used |

Enum options (label → our key): `XY` → `xy`, `ZY` → `zy` (default), `XZ` → `xz`.

## Cases

All inputs are a default cube (200 cm) at the listed position; `input.obj` in each folder.

| Case | Inputs and parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | cube child at position (150, 0, 0) | 16 / 12 (4-gons 12) / size (500, 200, 200), centre (0, 0, 0); normals 6, uvs 4 | original cube's 8 vertices first (x 50 … 250), then the mirrored 8 (x -50 … -250) |
| plane-xy | mirror plane = XY, cube at (0, 0, 150) | 16 / 12 (4-gons 12) / size (200, 200, 500), centre (0, 0, 0) | |
| plane-xz | mirror plane = XZ, cube at (0, 150, 0) | 16 / 12 (4-gons 12) / size (200, 500, 200), centre (0, 0, 0) | |
| weld | cube at (100, 0, 0) (its −X face on the mirror plane), weld points on (default tolerance) | 12 / 12 (4-gons 12) / size (400, 200, 200), centre (0, 0, 0) | the 4 points on the plane are shared (8 + 4 new vertices), but **both coincident faces at x = 0 are kept** (original `f 7 8 2 1` and mirrored `f 7 1 2 8`) |
| weld-off | cube at (100, 0, 0), weld points off | 12 / 12 (4-gons 12) / size (400, 200, 200), centre (0, 0, 0) | **byte-identical to weld**: the points on the plane are shared even with Weld Points off (the checkbox was confirmed off in the Attributes before export) |

## Behaviour notes
- Which parameters are greyed out or hidden by others: Weld Points off greys out
  Tolerance and Symmetrical; Delete Polygons on Axis is greyed by default.
- Anything surprising: the mirrored copy is appended after the original (same face
  order, reversed winding); points lying exactly on the mirror plane are merged
  regardless of Weld Points; the inner faces on the plane are not removed. With the
  cube away from the plane (defaults) nothing is merged.
