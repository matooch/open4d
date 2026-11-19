# Volume builder and mesher — observation (O32)

Status: observed (10 of 12 cases; subtract and intersect not scriptable) · App version: 25.010 (Maxon Cinema 4D R25) · OS: macOS 26.7.2 (25G332, Apple silicon) · Observer: Hermes · Date: 2026-11-19
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

What the script did (observed): the Volume Mesher menu command creates object id 1039861 and
the Volume Builder menu command creates id 1039859; the builder was inserted under the mesher
with `InsertUnder`, inputs were inserted under the builder the same way, and the mesher's cache
was resolved to a polygon object with `MCOMMAND_CURRENTSTATETOOBJECT` because
`GetCache()/GetDeformCache()` on the mesher return no `PolygonObject`. No separate "add to
list" call is needed: the builder picks up its direct children (a Null with two cube children
was traversed, see the hierarchy case).

## Parameters (fill once)

Defaults are the values read from a freshly created object: these parameters publish no
`DESC_DEFAULT`, so the "Default" column is the observed starting value, not a declared one.
Min/Max come from the description.

### Volume Builder
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Voxel Size | voxel_size | 10.0 | 0.001 | 1e+20 (step 0.1) |
| Interior Voxel Range | interior_voxel_range | 2 | 1 | 2147483647 |
| Exterior Voxel Range | exterior_voxel_range | 2 | 1 | 2147483647 |
| Spline Voxel Range | spline_voxel_range | 2 | 1 | 2147483647 |
| Particle Voxel Range | particle_voxel_range | 2 | 1 | 2147483647 |
| Volume Type | volume_type | Signed Distance Field (0) | options: Signed Distance Field (0), Fog (1), Vector (2) | – |
| Auto Update Settings | auto_update_settings | 1 | 0 | 1 |
| Objects | objects (id 6016) | empty | – | – |
| Update | update (id 6023) | button | – | – |
| Override Grid Matrix | override_grid_matrix (id 6026) | off | – | – |

Hidden rows that exist in the container but carry no label in the description: 6019 (string
status readout, e.g. "Voxel Amount: 7.254   Size: ~311 kb "), 6020 (int, 1), 6021 (int, 0).

Per-object settings in the list (mode, input type, …):
| Label | Our key | Default | Options |
|---|---|---|---|
| not observed | – | – | – |

not observed: the builder's description exposes no per-object rows at all — there is no mode
and no input-type parameter on either the builder or the child object. An object becomes a
list entry by being inserted under the builder; the per-entry operation mode (Union,
Subtract, Intersect) is a list-UI setting that the script API does not expose, and reading the
Objects list itself (`builder[6016]`) raises "Parameter value not accessible (object unknown in
Python)". The default that a plain insertion produces is Union: two overlapping spheres came
out as one merged surface (see union-two).

### Volume Mesher
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Surface Threshold | surface_threshold | 0.01 | -1e+20 | 1e+20 (step 0.01) |
| Voxel Range Threshold | voxel_range_threshold | 0.5 | 0.0 | 1.0 (step 0.01) |
| Use Absolute Value (ISO) | use_absolute_value_iso | off (0) | 0 | 1 |
| Adaptive | adaptive | 0.0 | 0.0 | 1.0 (step 0.001) |
| Create Curvature Map | create_curvature_map | off (0) | 0 | 1 |
| Curvature Direction | curvature_direction | Both (0) | options: Both (0), Convex (1), Concave (2) | – |

Surface Threshold is a plain float with no unit in the description (range −1e+20 … 1e+20), so
"the second step the field allows" was measured against its step of 0.01.

## Cases

| Case | Inputs | Builder / mesher changes | Notes |
|---|---|---|---|
| cube-defaults | a default cube | – | 2168 points, 2166 faces, all quads; bbox x/y/z ±100.0; OBJ 256926 bytes |
| cube-voxel-5 | a default cube | voxel size 5 | 9128 points, 9126 faces, all quads; bbox ±100.0; OBJ 1095284 bytes |
| cube-voxel-20 | a default cube | voxel size 20 | 488 points, 486 faces, all quads; bbox ±100.0; OBJ 57378 bytes |
| sphere-defaults | a default sphere | – | 1832 points, 1830 faces, all quads; bbox x ±97.996, y ±98.301, z ±97.996; OBJ 281179 bytes |
| sphere-voxel-5 | a default sphere | voxel size 5 | 7328 points, 7326 faces, all quads; bbox x ±98.998, y ±99.151, z ±98.998; OBJ 1161715 bytes |
| union-two | sphere at (−60, 0, 0) and sphere at (60, 0, 0), both Union | – | 2952 points, 2950 faces, all quads; bbox x ±157.996 (the two spheres merged into one surface, the overlap is gone); OBJ 457253 bytes |
| subtract | cube, then sphere at (100, 100, 0) set to Subtract | – | not observed: the per-object operation mode is not exposed in the builder's description and the Objects list (id 6016) cannot be read from Python ("Parameter value not accessible (object unknown in Python)"), so the sphere could not be switched to Subtract |
| intersect | cube, then sphere at (50, 0, 0) set to Intersect | – | not observed: same reason as subtract — no scriptable way to set a list entry's mode to Intersect |
| mesher-threshold | a default sphere | mesher threshold (iso value) changed to the second step the field allows, e.g. 0.5 or 5 | 1832 points, 1830 faces, all quads; the OBJ is byte-identical to sphere-defaults (md5 769066acf9ecf2397d7e4a0120f896f4), so Surface Threshold 0.5 changed nothing at voxel size 10. Unit: plain float, range −1e+20 … 1e+20, step 0.01 |
| mesher-adaptive | a default sphere | mesher adaptive on / adaptivity 50 % (or the closest settings) | Adaptive set to 0.5 (the range is 0.0 … 1.0): 53 points, 72 faces, 30 quads and 42 triangles — the only case with triangles, and it drops detail (bbox shrinks to x ±88.071, y ±88.435); OBJ 7634 bytes |
| torus-defaults | a default torus | – | 1400 points, 1398 faces, all quads; bbox x/y/z ±83.07; OBJ 203275 bytes |
| hierarchy | a Null containing two cubes at x ±80, the Null in the list, with "use hierarchy" (or the closest option) | – | 3384 points, 3382 faces, all quads; bbox x ±180.0 (both cubes contributed, so the Null's children were traversed); OBJ 405692 bytes. No "use hierarchy" parameter exists in the builder's description — inserting the Null under the builder was enough |

Record the vertex and face counts and whether the faces are quads or triangles.

## Notes

- Every case except mesher-adaptive comes out with quads only; adaptive produces a mix
  (30 quads, 42 triangles in the sphere case).
- Point count follows the voxel size as expected for the same cube: 488 points at voxel 20,
  2168 at 10, 9128 at 5. The mesh is re-tessellated, not decimated.
- Surface Threshold has no visible effect at voxel size 10 between 0.01 and 0.5: the exported
  meshes are byte-identical (md5 769066acf9ecf2397d7e4a0120f896f4 for both).
- The builder's status readout (hidden parameter 6019) reported "Voxel Amount: 7.254   Size:
  ~311 kb " for the default cube scene, i.e. the builder reports its own grid cost.
- Export used the usual OBJ settings through the mesher; `case.toml` files also carry the
  generic object rows (position, rotation, scale, colour) of each input object, exported from
  the description, plus the `[import]` block with scale 1.0 and no Z flip.
- The ten observed cases are committed under `tests/golden/volumes/<case>/` with `mesh.obj` and
  `case.toml`; subtract and intersect have no golden data because their mode could not be set.
