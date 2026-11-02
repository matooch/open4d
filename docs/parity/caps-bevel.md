# Caps and bevel tab (Extrude, Lathe, Loft, Sweep) — observation (O56)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted. Every case is an Extrude of a default Rectangle spline, Offset 100,
**unless the row says otherwise**.

Output: `tests/golden/caps-bevel/<case>/mesh.obj`, with `feature = "caps-bevel"`. `case.toml`
holds every parameter on the generator's Object, Caps and Bevel tabs.

Unblocks: C/044, 045.

## Tabs (fill once)

Record the Caps tab and the Bevel tab with label, our key, default and options.
O4 noted that these tabs are shared by Extrude, Lathe, Loft and Sweep in R25.

## Cases

| Case | Changes |
|---|---|
| defaults | – |
| direction-x | Extrude Direction X (or the first non-Auto option), Offset 100 |
| direction-custom | Direction Custom with movement (0, 50, 100), if offered |
| caps-triangles | Caps Type Triangles |
| caps-quads | Caps Type Quadrangles |
| caps-ngon | Caps Type N-gon (the default?) |
| caps-delaunay | Caps Type Delaunay |
| caps-grid | Caps Type Regular Grid, grid width 20 |
| start-only | Start Cap on, End Cap off |
| bevel-chamfer | Bevel on both caps: Chamfer, size 10, segments 1 |
| bevel-round | Bevel Round, size 10, segments 3 |
| bevel-concave | Bevel Concave (or the third shape), size 10, segments 3 |
| bevel-step | Bevel Step, size 10, segments 2 |
| bevel-inner | Bevel size 10 with "inside"/"outside" switched (if the option exists) |
| lathe-caps | Lathe of a default Rectangle at x 300, angle 180°, caps on (default type) |
| sweep-caps-bevel | Sweep with a default Circle profile on an open Arc path: caps on, bevel round 5 |

## Notes
TODO
