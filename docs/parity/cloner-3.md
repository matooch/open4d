# Cloner III — observation (O21)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: exactly as [cloner.md](cloner.md) (O8). The cloner is made through its menu
command, and its children are 20 cm cubes. Save the clone data (`samples.json`) and
`mesh.obj` to `tests/golden/cloner/<case>/`, with
`observation = "docs/parity/cloner-3.md"`.

Unblocks: P3 cloner modes in depth.

## Cases

| Case | Mode | Changed from default | Notes |
|---|---|---|---|
| object-edge | Object, on a default sphere (8 segments) | distribution Edge | |
| object-polycenter | Object, on the same sphere | distribution Polygon Center | |
| object-surface | Object, on the same sphere | distribution Surface, count 20 | Record the seed |
| object-surface-seed | Object, on the same sphere | Surface, count 20, seed 42 | |
| object-volume | Object, on a default cube | distribution Volume, count 20 | Record the seed |
| object-axis | Object, on a default cube | distribution Axis | |
| object-vertex-align-off | Object, on the sphere | Vertex, Align Clone off | |
| grid-sphere-form | Grid Array | form Sphere, count (5, 5, 5) | |
| grid-cylinder-form | Grid Array | form Cylinder, count (5, 5, 5) | |
| grid-fill-50 | Grid Array | form Sphere, count (5, 5, 5), fill 50 % | |
| grid-endpoint | Grid Array | mode Endpoint, size (300, 300, 300) | |
| honeycomb-small | Honeycomb | count width 3, count height 2 | |
| honeycomb-offset | Honeycomb | count 3 × 2, offset direction Width, offset 25 % | |
| linear-cumulative | Linear | count 5, step mode Cumulative, step rotation H 15° | |
| linear-scale-step | Linear | count 5, scale per step 90 % | |
| radial-align-off | Radial | Align off | |
| radial-offset | Radial | offset 30°, count 8 | |
| order-iterate | Linear, count 6, **three children**: a cube, a sphere (radius 10) and a cone (radius 10) | clones mode Iterate | Which child each clone uses: record it per clone as `child` (0, 1 or 2) in the clones entry, if it can be read |
| order-random | the same | clones mode Random (record the seed) | |
| order-sort | the same | clones mode Sort | |
| nested | Linear, count 3, whose child is a Radial cloner (count 4) with a cube child | – | Record the outer cloner's clones and export the whole mesh |

## Notes
TODO
