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

## Notes
TODO
