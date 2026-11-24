// Acceptance scenario S5 (docs/roadmap/README.md): volumes through the CLI.
// Run: forge run tests/acceptance/s5_volume.rhai --save s5.forge
//
// Three spheres of radius 100 with centres 120 apart on the X axis, and a Circle spline of
// radius 300 in the XZ plane whose tube radius comes from the builder's Spline Voxel Range
// (0.735 * 4 * 10 = 29.4, docs/specs/VOLUME.md 6.3), are children of one Volume Builder in
// Union mode (the default). The Gaussian smooth layer is the list's first entry, so it acts on
// every entry below it (VOLUME.md 6.2); the Volume Mesher on top meshes the union and Make
// Editable turns the result into one polygon object. Nothing is animated.
//
// The spheres use 64 segments so their faceted volume is within 0.4 % of the analytic sphere
// (at the 16-segment default it is 6.3 % low, which would hide in the volume tolerance). The
// smooth layer is deliberately light (Voxel Distance 1, Strength 0.1): the default width and
// strength over-smooth a 3-voxel tube by ~20 % in volume and produce 16 non-manifold edges.
run("timeline.settings", #{ fps: 30, start: 0, end: 0 });

let mesher = create("forge.volume_mesher", "Volume Mesher");
let builder = create("forge.volume_builder", "Volume Builder");
run("object.reparent", #{ node: builder, parent: mesher });
set_param(builder, "voxel_size", 10.0);
set_param(builder, "spline_voxel_range", 4);

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
let s2 = create("forge.sphere", "Sphere 2");
run("object.reparent", #{ node: s2, parent: builder });
set_param(s2, "radius", 100.0);
set_param(s2, "segments", 64);
set_transform(s2, #{ pos: vec3(0, 0, 0) });
let s3 = create("forge.sphere", "Sphere 3");
run("object.reparent", #{ node: s3, parent: builder });
set_param(s3, "radius", 100.0);
set_param(s3, "segments", 64);
set_transform(s3, #{ pos: vec3(120, 0, 0) });

let ring = create("forge.circle_spline", "Circle");
run("object.reparent", #{ node: ring, parent: builder });
set_param(ring, "plane", "xz");
set_param(ring, "radius", 300.0);
set_param(ring, "radius_y", 300.0);

// The mesher's mesh becomes a polygon object; the volume nodes are consumed and gone.
let made = run("mesh.make_editable", #{ nodes: [mesher] });
let polygon = made.made[0];
print(`polygon=${polygon} builder=${builder} mesher=${mesher}`);
polygon
