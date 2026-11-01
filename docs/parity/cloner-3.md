# Cloner III — observation (O21)

Status: observed (21 of 21 cases; the per-clone child index could not be read, see Notes) · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: exactly as [cloner.md](cloner.md) (O8). The cloner is made through its menu
command, and its children are 20 cm cubes. Save the clone data (`samples.json`) and
`mesh.obj` to `tests/golden/cloner/<case>/`, with
`observation = "docs/parity/cloner-3.md"`.

Unblocks: P3 cloner modes in depth.

## Cases

Clone counts are the number of entries in `samples.json` (visible = the clone-on flag).

| Case | Mode | Changed from default | Notes |
|---|---|---|---|
| object-edge | Object, on a default sphere (8 segments) | distribution Edge | 56 clones, all visible. The sphere stays in the scene, so `mesh.obj` holds it too |
| object-polycenter | Object, on the same sphere | distribution Polygon Center | 32 clones |
| object-surface | Object, on the same sphere | distribution Surface, count 20 | 20 clones. **Seed 1234567** (the default; Surface is the default distribution after linking, Count default 20) |
| object-surface-seed | Object, on the same sphere | Surface, count 20, seed 42 | 20 clones; positions differ from object-surface (first clone (76.56, −22.815, 33.774) against (66.963, 38.948, −40.81)) |
| object-volume | Object, on a default cube | distribution Volume, count 20 | 20 clones. **Seed 1234567** (default) |
| object-axis | Object, on a default cube | distribution Axis | 1 clone, at (0, 0, 0) |
| object-vertex-align-off | Object, on the sphere | Vertex, Align Clone off | 26 clones, one per vertex; first at (0, −100, 0). Align Clone reads back off |
| grid-sphere-form | Grid Array | form Sphere, count (5, 5, 5) | 125 clones, **33 visible**: clones outside the sphere stay in the list but their visible flag is off. Size stays (200, 200, 200) |
| grid-cylinder-form | Grid Array | form Cylinder, count (5, 5, 5) | 125 clones, **65 visible** |
| grid-fill-50 | Grid Array | form Sphere, count (5, 5, 5), fill 50 % | 125 clones, **32 visible** (against 33 at fill 100 %) |
| grid-endpoint | Grid Array | mode Endpoint, size (300, 300, 300) | Count stays at its default (3, 1, 3): 9 clones from (−150, 0, −150) to (150, 0, 150) with step 150 |
| honeycomb-small | Honeycomb | count width 3, count height 2 | 3 clones at (−300, −86.603, 0), (0, 86.603, 0), (300, −86.603, 0) (not 6) |
| honeycomb-offset | Honeycomb | count 3 × 2, offset direction Width, offset 25 % | 3 clones. **Setting Offset Direction to Width swaps the two counts and the two sizes** (Width 3 × Height 2 became 2 × 3, size 346.41 × 300). The probe therefore sets the direction first and the counts after; the final values are count_width 3, count_height 2, size_width 346.41, size_height 300 |
| linear-cumulative | Linear | count 5, step mode Cumulative, step rotation H 15° | 5 clones; step mode and step rotation read back as set |
| linear-scale-step | Linear | count 5, scale per step 90 % | The "S (per step)" field, 90 % on all three axes; step mode stays Single Value |
| radial-align-off | Radial | Align off | 5 clones; first at (0, 0, 50) |
| radial-offset | Radial | offset 30°, count 8 | 8 clones; first at (25, 0, 43.301). Offset Seed 1234567 (default) |
| order-iterate | Linear, count 6, **three children**: a cube, a sphere (radius 10) and a cone (radius 10) | clones mode Iterate | 6 clones at y 0 … 250. `child`: **TODO**, see Notes |
| order-random | the same | clones mode Random | Seed reads **123456**. `child`: TODO |
| order-sort | the same | clones mode Sort | `child`: TODO |
| nested | Linear, count 3, whose child is a Radial cloner (count 4) with a cube child | – | `samples.json` holds the outer cloner's 3 clones (y 0, 50, 100); `mesh.obj` holds the whole scene |

## Notes

- **Clones menu values.** The Clones cycle stores Iterate = 1, Random = 2, Blend = 3, Sort = 4 (not 0, 1, 2, 3). Setting 0 is refused ("parameter set failed"). A first run of the order cases used 0/1/3 and got the wrong modes (Iterate error, Iterate, Blend); those were redone with the right values and the files overwritten.
- **Which child each clone uses (`child`) is TODO.** The clone-data call has a per-clone clone-index array, but it reads 0 for every clone in all three order cases, including Random and Sort, so it is not the child index. No `child` field was written to `samples.json`. The choice per clone is visible only in `mesh.obj` (the cube, sphere and cone have different vertex counts), which holds the answer if someone reads it from there.
- **The clone positions of order-iterate, order-random and order-sort are identical** (same Linear layout); the modes differ only in which child each clone shows.
- Grid forms other than Cubic hide clones through the visible flag; the list keeps all 125 entries, so a consumer must check `visible`.
- Object-mode `mesh.obj` files include the source object (sphere or cube) as well as the clones.
- Honeycomb size defaults change with the offset direction (Height: 300 × 346.41, Width: 346.41 × 300).
- Not checked: Seed of Random clones mode other than its default value; Blend mode; the cloner's own `child` order under Sort with weights.
