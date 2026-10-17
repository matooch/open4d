# Cube primitive — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-17
Exporter settings: [export-settings.md](export-settings.md). Folder: `tests/golden/cube/<case>/`.

Created with the toolbar Cube button in a new scene (File > New). Parameters are in the
Attributes panel, "Object" tab (tabs: Basic, Coord., Object, Phong), group "Object
Properties". Layout: Size . X / Size . Y / Size . Z in the left column, Segments X / Y / Z
in the right column, then Separate Surfaces, Fillet, Fillet Radius, Fillet Subdivision.

## Parameters (as shown in the app)

Min/max: found by typing 999999999 (or 999999 for ints) and a negative value into the
field, then reading back what it shows.

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Size . X / Size . Y / Size . Z | `size_x` / `size_y` / `size_z` | float | 200 / 200 / 200 | 0 (-100 typed → 0 cm) | none seen (999999999 accepted; the Coordinates manager showed 999999999 cm) | cm | |
| Segments X / Segments Y / Segments Z | `segments_x` / `_y` / `_z` | int | 1 / 1 / 1 | 1 (-5 typed → 1) | 1000 (999999 typed → 1000) | none | |
| Separate Surfaces | `separate_surfaces` | bool | off | – | – | – | checkbox; greyed out (disabled) while Fillet is on. Not used in any case. |
| Fillet | `fillet` | bool | off | – | – | – | checkbox; Fillet Radius and Fillet Subdivision are greyed out while it is off |
| Fillet Radius | `fillet_radius` | float | 40 | 0 (-10 typed → 0 cm) | half the smallest size (999 typed with sizes 200/300/200 → 100 cm; 999999999 typed with Size . Y = 0 → 0 cm) | cm | |
| Fillet Subdivision | `fillet_segments` | int | 5 | 1 (-5 typed → 1) | 1000 (999999 typed → 1000) | none | |

## Cases

| Case | Set | `golden info` (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 8 / 6 (4-gons 6) / size (200, 200, 200), centre (0, 0, 0) | normals 6, uvs 4 |
| sizes | size = (100, 200, 300) | 8 / 6 (4-gons 6) / size (100, 200, 300), centre (0, 0, 0) | normals 6, uvs 4 |
| segments | segments = (2, 3, 4) | 54 / 52 (4-gons 52) / size (200, 200, 200), centre (0, 0, 0) | normals 6, uvs 29. Exported as-is. |
| segments-wide | segments = (3, 2, 3) | 44 / 42 (4-gons 42) / size (200, 200, 200), centre (0, 0, 0) | normals 6, uvs 20. Exported as-is; added so the top/bottom faces have more than one interior column. |
| fillet | fillet on, radius = 20, fillet segments = 3 | 104 / 114 (3-gons 24, 4-gons 90) / size (200, 200, 200), centre (0, 0, 0) | normals 105, uvs 44 |
| fillet-segments | fillet on + segments = (2, 2, 2) | 326 / 344 (3-gons 40, 4-gons 304) / size (200, 200, 200), centre (0, 0, 0) | Fillet Radius and Fillet Subdivision left at their defaults (40, 5). normals 327, uvs 113 |

## Behaviour notes
- What happens at segments = 1 vs the minimum allowed: 1 is the minimum; typing 0 or
  a negative value gives 1. With segments (1, 1, 1) the cube exports as 8 vertices /
  6 quads.
- Does fillet radius clamp relative to size? Yes. The field is clamped to half of the
  smallest Size value (sizes 200/300/200 → max 100 cm; a Size of 0 → max 0 cm). Min 0.
- Separate Surfaces is disabled while Fillet is on.
- The exporter writes one `vn` per face for an unfilleted cube (6), but about one per
  vertex once fillet is on (105 for 104 vertices).
