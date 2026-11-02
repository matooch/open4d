# Clone-toolset generators II — observation (O31)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted. Each generator is made through its menu command; children are
inputs, as named. Export the whole result as `tests/golden/<feature>/<case>/mesh.obj`, and
also save the clone data (`samples.json`, kind `clones`, as [cloner.md](cloner.md)) where
the generator has clones. Use `observation = "docs/parity/clone-generators-2.md"`.

Don't use the clone toolset's brand-style names in folder names or keys. Use the feature
names given below.

Unblocks: P3 (Matrix, Fracture, Voronoi Fracture, the text generator, Tracer).

## Parameters (fill once per generator, as in cloner.md)

### Matrix (`matrix`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Fracture (`fracture`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Voronoi Fracture (`voronoi-fracture`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Text generator of the clone toolset (`clone-text`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Tracer (`tracer`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case (folder) | Generator | Inputs | Changed | Output |
|---|---|---|---|---|
| matrix/defaults | Matrix | – | – | samples only (no geometry); also export a 20 cm cube at the origin so the case has a mesh |
| matrix/linear | Matrix | – | mode Linear, count 5 | the same |
| fracture/explode-segments | Fracture | a default cube, then Make Editable and Disconnect every polygon | mode Explode Segments | mesh and samples |
| fracture/plain | Fracture with a Plain effector P (0, 100, 0) | the same | mode Explode Segments | mesh and samples |
| voronoi-fracture/defaults | Voronoi Fracture | a default cube | – (record the point source and its seed) | mesh |
| voronoi-fracture/count-10 | Voronoi Fracture | a default cube | point count 10, seed 1 | mesh |
| voronoi-fracture/sphere | Voronoi Fracture | a default sphere | point count 10, seed 1 | mesh |
| clone-text/defaults | the text generator | – | – | mesh. Record the font name shown. If the font is not a system default, use a plain system sans font and record which |
| clone-text/depth | the text generator | – | depth 50, text `AB` | mesh |
| tracer/two-nulls | Tracer | two Nulls keyed from (0, 0, 0) at 0 to (100, 100, 0) at 30 and from (100, 0, 0) to (200, 100, 0) | trace mode Connect All Objects | export at frame 30 through an Extrude, as in [splines-2.md](splines-2.md) |

## Notes
TODO
