# Viewport default camera — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Unblocks: showing the reference app's left-handed world un-mirrored in our viewport
(docs/specs/TRANSFORM.md §1, VIEWPORT.md §1). No mesh export needed.

Record **where things appear on screen**, not how the viewport looks (our look stays
our own). Screenshots may be taken to read positions but are not committed.

## Setup
New scene (File > New). Add three markers, each a default cube scaled down so they are
easy to tell apart, or three different primitives:

| Marker | Object | Position typed |
|---|---|---|
| A | default cube, Size 40 | (200, 0, 0) |
| B | default sphere, Radius 20 | (0, 200, 0) |
| C | default cone (or a cube with Size 20) | (0, 0, 200) |

## Default perspective view (do not move the camera)

| Question | Answer |
|---|---|
| Viewport name/label shown (e.g. "Perspective") | TODO |
| Marker A (+X): left or right of the origin on screen? | TODO |
| Marker B (+Y): above or below the origin? | TODO |
| Marker C (+Z): which way on screen, and is it nearer to or farther from the camera than the origin (appears larger/lower = nearer)? | TODO |
| Is the camera looking down at the origin from above, from below, or level? | TODO |
| Default editor camera position/rotation, if the app shows them anywhere (e.g. view or camera attributes) | TODO / not shown |
| Corner axis icon: which way each of its X, Y, Z arms points | TODO |

## Front / side / top views (if the app offers them)

For each standard view the app has (menu labels as shown), the same three questions:
which way A (+X), B (+Y) and C (+Z) appear (left/right/up/down, or toward/away from
the viewer when a marker hides behind or in front of the origin).

| View (label) | +X appears | +Y appears | +Z appears |
|---|---|---|---|
| TODO | | | |

## Navigation (if not already recorded in defaults.md)
Mouse/keys for orbit, pan and zoom in the perspective view, and which wheel direction
zooms in: TODO
