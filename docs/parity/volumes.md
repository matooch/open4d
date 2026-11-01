# Volume builder and mesher — observation (O32)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted ([../agents/OBSERVER.md](../agents/OBSERVER.md), [QUEUE.md](QUEUE.md) rules).
Unblocks: F/080, 081, 083 (`docs/roadmap/areas/F-volumes.md`). Parity is geometric (Hausdorff
distance below voxel size / 2).

## Setup for every case

1. New document.
2. Make a **Volume Mesher** through its menu command. Make a **Volume Builder** through its
   menu command and put it as the mesher's child.
3. Make the input objects through their menu commands and add them to the builder's object
   list in the order given. Use the builder's own add mechanism (dragging the objects
   under the builder or into its list), whichever the script can drive.
4. Export the mesher as OBJ with the usual settings to `tests/golden/volumes/<case>/mesh.obj`.

`case.toml`:
- `feature = "volumes"`, with `observation = "docs/parity/volumes.md"`;
- `[params.builder]`: every builder parameter (voxel size, the per-object mode and input
  type, etc.);
- `[params.mesher]`: every mesher parameter;
- `[params.input]`, `[params.input2]` …: the input objects with all their parameters and
  positions.

## Parameters (fill once)

### Volume Builder
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

Per-object settings in the list (mode, input type, …):
| Label | Our key | Default | Options |
|---|---|---|---|

### Volume Mesher
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case | Inputs | Builder / mesher changes | Notes |
|---|---|---|---|
| cube-defaults | a default cube | – | |
| cube-voxel-5 | a default cube | voxel size 5 | |
| cube-voxel-20 | a default cube | voxel size 20 | |
| sphere-defaults | a default sphere | – | |
| sphere-voxel-5 | a default sphere | voxel size 5 | |
| union-two | sphere at (−60, 0, 0) and sphere at (60, 0, 0), both Union | – | |
| subtract | cube, then sphere at (100, 100, 0) set to Subtract | – | |
| intersect | cube, then sphere at (50, 0, 0) set to Intersect | – | |
| mesher-threshold | a default sphere | mesher threshold (iso value) changed to the second step the field allows, e.g. 0.5 or 5 | Record the field's unit |
| mesher-adaptive | a default sphere | mesher adaptive on / adaptivity 50 % (or the closest settings) | |
| torus-defaults | a default torus | – | Checks genus 1 |
| hierarchy | a Null containing two cubes at x ±80, the Null in the list, with "use hierarchy" (or the closest option) | – | |

Record the vertex and face counts and whether the faces are quads or triangles.

## Notes
TODO
