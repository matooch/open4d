# Volumes

Volumes build a continuous field — a signed distance or a density — from ordinary objects,
smooth and filter it, and mesh the result back into polygons. The three node types are the
**Volume Builder** (`forge.volume_builder`), the **Volume Filter** (`forge.volume_filter`) and
the **Volume Mesher** (`forge.volume_mesher`); *Create › Volume* adds them. The field rules are
in [VOLUME.md](../specs/VOLUME.md), the parameters in the reference below.

## The object list

A builder reads its **children, top to bottom**. Objects, splines, clones and other builders
become entries; the top entry is applied last, so a filter placed at the top of the list acts
on everything below it. Spheres, cubes and meshes use their distance field, a spline becomes a
tube of radius `0.735 * Spline Voxel Range * Voxel Size` (29.4 for a range of 4 at voxel
size 10), and each entry blends as Union unless an entry row in the `objects` list says
Subtract or Intersect.

`Voxel Size` is the resolution of the field and the mesher: halve it to double the detail and
roughly quadruple the build time. The ranges (`Interior`, `Exterior`, `Spline`) are bands in
voxels around a surface; a spline tube needs a Spline Voxel Range large enough for its radius.

Filters are children of the builder too. `offset` moves the surface, `reshape` opens and
closes it, and `smooth` runs a Gaussian, mean, median or Laplacian operator; `Strength` blends
the filtered field back into the input and `Voxel Distance` is the window. Strong smoothing of
a thin tube can pinch it: keep the strength low, or widen the tube.

## Meshing and editing

The Volume Mesher takes the first builder below it, meshes the iso-surface (`Surface
Threshold`, `Adaptive` for reduced polygon counts) and returns ordinary polygons. **Make
Editable** on the mesher (`mesh.make_editable`) replaces the whole volume stack with one
polygon object, which you can then edit, export or hand to a deformer. Consumed inputs are not
drawn or exported while they are still under a mesher.

The mesher's vertices are cell means, so the surface sits up to about one voxel inside the
sources; a sphere of radius 100 meshed at voxel size 10 has a volume about 1.5 % below the
analytic one.

## Walkthrough: an S5 scene from a script

The S5 acceptance scenario of the roadmap builds a smooth union of three spheres and a spline
tube, meshes it and makes it editable in one Rhai script:
[`tests/acceptance/s5_volume.rhai`](../../tests/acceptance/s5_volume.rhai). The acceptance test
runs exactly that file through the CLI, so the script doubles as an automation example.

```rhai
let mesher = create("forge.volume_mesher", "Volume Mesher");
let builder = create("forge.volume_builder", "Volume Builder");
run("object.reparent", #{ node: builder, parent: mesher });
set_param(builder, "voxel_size", 10.0);
set_param(builder, "spline_voxel_range", 4);

// The first child is the top of the list: it filters every entry below it.
let smooth = create("forge.volume_filter", "Smooth");
run("object.reparent", #{ node: smooth, parent: builder });
set_param(smooth, "kind", "smooth");
set_param(smooth, "smooth_operator", "gaussian");
set_param(smooth, "voxel_distance", 1);
set_param(smooth, "strength", 0.1);
set_param(smooth, "iterations", 1);

let s1 = create("forge.sphere", "Sphere 1");
run("object.reparent", #{ node: s1, parent: builder });
set_param(s1, "radius", 100.0);
set_param(s1, "segments", 64);
set_transform(s1, #{ pos: vec3(-120, 0, 0) });
// ... two more spheres at x = 0 and x = 120, then the Circle spline:
let ring = create("forge.circle_spline", "Circle");
run("object.reparent", #{ node: ring, parent: builder });
set_param(ring, "plane", "xz");
set_param(ring, "radius", 300.0);

// One polygon object; the volume nodes are consumed.
let made = run("mesh.make_editable", #{ nodes: [mesher] });
made.made[0]
```

Save the result with `forge run tests/acceptance/s5_volume.rhai --save s5.forge`, then export
the polygon with `forge export s5.forge --format obj --out s5.obj` or evaluate it at a frame
with `forge eval s5.forge --export s5.obj`. See [Automation](automation.md) for the script API
and [Files and recovery](files-and-recovery.md) for saving and reopening scenes.

## What the field does not hold

Fog volumes store density rather than distance; the mesher then uses the threshold as a
density cut. Vector volumes and `Override Grid Matrix` are not supported. VDB files are read
and written by the same grid code (see [VOLUME.md](../specs/VOLUME.md) section 10).
