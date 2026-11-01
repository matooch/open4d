# Clone toolset III — observation (O50)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: as [cloner.md](cloner.md) (O8) for clone data (`samples.json`, kind `clones`) and
meshes. Spline outputs go through an Extrude as in [splines-2.md](splines-2.md).

Folders:
- `tests/golden/cloner/<case>/` for cloner cases;
- `tests/golden/clone-spline/<case>/` for the spline generator;
- `tests/golden/tracer/<case>/` for Tracer.

Unblocks: E/067, 076, 077, 078.

## Cases

| Case | Setup | Output |
|---|---|---|
| cloner/blend-order | Linear, count 5; children a cube (size 20) and a sphere (r 10); clones mode Blend | samples + mesh |
| cloner/selection-tag | Linear, count 5, plus a clone selection tag selecting clones 1 and 3, and a Plain effector (P 0, 100, 0) restricted to that selection | samples + mesh |
| cloner/weight-tag | Linear, count 5, with a clone weight tag (weights painted or set 0, 0.25, 0.5, 0.75, 1 by script) and a Plain effector using weight | samples + mesh |
| cloner/on-spline | Object mode on a default Helix, count 10, distribution Count | samples + mesh |
| cloner/on-spline-step | the same, distribution Step 50 | samples + mesh |
| clone-spline/simple | the clone-toolset spline generator in Simple mode, defaults | extruded mesh |
| clone-spline/turtle | Turtle mode, defaults | extruded mesh |
| clone-spline/spline-mode | Spline mode with a default Circle as source | extruded mesh |
| tracer/vertices | Tracer, trace mode "vertices of object", on a cube keyed 0 → 300 in x over frames 0–30; exported at frame 30 through an Extrude | mesh |
| tracer/interval | the same, with the interval set to 5 | mesh |
| instance-time/defaults | the instance-over-time generator on a cube keyed as above, defaults, at frame 30 | samples + mesh |
| cloner-f/transform-rs | follow-up from brief 065: Linear cloner (count 3, offset (0, 0, 100)) of a 20 cm cube; Transform tab R (0, 30, 0), S (1.5, 1, 1), colour (1, 0, 0), weight 50 % | samples |
| cloner-f/linear-offset | Linear, count 5, P (0, 50, 0), Offset 1, then a second case `linear-offset-15` with Offset 1.5 | samples |
| cloner-f/linear-amount | Linear, count 5, P (0, 50, 0), Amount 50 % | samples |
| cloner-f/linear-rstep | Linear, count 5, P (0, 50, 0), per-step R (0, 20, 0) with Step Rotation H 10° | samples |
| cloner-f/linear-endpoint-sr | Linear End Point mode, count 5, P (0, 200, 0), S (0.5, 0.5, 0.5), R (0, 90, 0) | samples |
| cloner-f/radial-var-1 | Radial, count 8, Offset Variation 50 %, seed 1; and `radial-var-2` with seed 2 | samples |
| cloner-f/grid-cyl-fill | Grid Array, 5/5/5, form Cylinder, Fill 50 % | samples |
| cloner-f/grid-sphere-uneven | Grid Array, counts 7/5/3, size (300, 200, 100), form Sphere | samples |
| cloner-f/honey-orient-x | Honeycomb, orientation X (if offered), defaults otherwise; and `honey-orient-y` | samples |
| cloner-f/honey-endpoint | Honeycomb, End Point mode, defaults otherwise | samples |
| cloner-f/honey-var | Honeycomb, both variations 50 %, seed 1 | samples |
| cloner-f/honey-odd-even | Honeycomb, width count 5, height count 4 | samples |
| cloner-f/reset-coords-off | Linear, count 3, the child cube moved to (0, 50, 0) and rotated H 30°, Reset Coordinates off | samples + mesh |

## Notes
TODO
