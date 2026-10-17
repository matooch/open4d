# Viewport default camera — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-17
Unblocks: showing the reference app's left-handed world un-mirrored in our viewport
(docs/specs/TRANSFORM.md §1, VIEWPORT.md §1). No mesh export needed.

Record **where things appear on screen**, not how the viewport looks (our look stays
our own). Screenshots may be taken to read positions but are not committed.

## Setup
New scene (File > New). Three markers, values typed in the Attributes panel:

| Marker | Object | Position typed |
|---|---|---|
| A | default cube, Size 40 / 40 / 40 | (200, 0, 0) |
| B | default sphere, Radius 20 | (0, 200, 0) |
| C | default cube, Size 20 / 20 / 20 (used instead of a cone) | (0, 0, 200) |

Screen positions below are in the app's single default viewport (about 920 × 610 px
on a 1389 × 868 screen). The world origin was located from the move gizmo of an
object at (0, 0, 0) in the same default view: about the middle of the viewport, a
little right of and below centre.

## Default perspective view (do not move the camera)

| Question | Answer |
|---|---|
| Viewport name/label shown (e.g. "Perspective") | "Perspective" (top-left corner of the viewport) |
| Marker A (+X): left or right of the origin on screen? | Right, and a little below the origin (about 170 px right, 55 px down). It is drawn the largest of the three (40 cube about 70 px wide). |
| Marker B (+Y): above or below the origin? | Directly above (same screen x, about 205 px up). |
| Marker C (+Z): which way on screen, and is it nearer to or farther from the camera than the origin (appears larger/lower = nearer)? | Up and to the right (about 125 px right, 40 px up). It appears small (20 cube about 24 px wide) and higher than the origin, so **farther** from the camera than the origin. |
| Is the camera looking down at the origin from above, from below, or level? | From above: the top face of a default cube at the origin is visible. |
| Default editor camera position/rotation, if the app shows them anywhere (e.g. view or camera attributes) | not shown (the Attributes Mode menu has no camera entry; the viewport Cameras menu lists views, not coordinates) |
| Corner axis icon: which way each of its X, Y, Z arms points | The icon (bottom-left of the viewport, about 25 px) is too small to read exactly. Its Y arm points up; X and Z arms point to the right, matching the object gizmo, whose arms point: Y (green) straight up, X (red) right and slightly down, Z (blue) right and up. |

## Front / side / top views (if the app offers them)

Viewport menu "Cameras" lists: Navigation ▸, Use Camera ▸, Set Active Object as Camera,
Camera 2D Mode, Perspective, Parallel, Left, Right, Front, Back, Top, Bottom,
Axonometric ▸. The viewport label changes to the view name. All six below are
parallel (no perspective size change), so a marker on the view axis just sits at the
origin's screen spot; whether it is toward or away from the viewer is not visible.

| View (label) | +X appears | +Y appears | +Z appears |
|---|---|---|---|
| Front | right | up | on the view axis (at the origin's spot) |
| Back | left | up | on the view axis |
| Left | on the view axis | up | left |
| Right | on the view axis | up | right |
| Top | right | on the view axis | up (screen up) |
| Bottom | right | on the view axis | down (screen down) |

Switching back with Cameras > Perspective restored exactly the original default view.

## Navigation (if not already recorded in defaults.md)
- Mouse wheel over the perspective view: wheel **up zooms in** (markers got larger),
  wheel down zooms out.
- Orbit / pan / drag-zoom with mouse buttons plus modifier: not tested (the observer
  tool cannot hold a modifier key or the middle/right button during a drag). The
  viewport has four small navigation icons at its top-right corner; not used.
