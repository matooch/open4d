# Deformers IV — observation (O45)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-11-04
Method: as [deformers-2.md](deformers-2.md) (O9). Unless a row says otherwise:
- the input is a cube with size_y 400 and segments 4/8/4;
- the deformer is the cube's child;
- export to `tests/golden/<deformer>/<case>/mesh.obj`.

Linked helper objects (splines, cages, surfaces, cameras) are made through their menu
commands. Record each helper's parameters under `[params.helper]` in `case.toml`.

Unblocks: D/055, 056.

Record each deformer's parameter table once.

## Cases

| Case (folder) | Deformer | Setup / changes |
|---|---|---|
| spline-deformer/defaults | Spline | original spline: a default Arc; modifier spline: the same Arc rotated H 90°; both linked |
| spline-rail/defaults | Spline Rail | Rail splines: two default Helix splines, one at x −150 and one at x 150 |
| mesh-deformer/cage | Mesh (cage) | cage: a cube of size 300 × 500 × 300 with segments 1/1/1, made editable, linked and initialised (the "initialize" button or the scripting equivalent); then move cage point 0 by (−100, 0, 0) |
| surface/defaults | Surface | surface: a default plane at y 250, linked, initialised; then bend the plane with a Bend deformer (strength 90) |
| camera-deformer/grid | Camera | a camera at (0, 0, −800) linked, default grid; move grid point (1, 1) by (50, 50) in the deformer's own grid space |
| collision/sphere | Collision | colliders: a sphere (r 100) at (150, 0, 0) in its list, defaults |
| jiggle/frame-10 | Jiggle | the cube keyed in position X 0 → 300 over frames 0–10; frame-stepped from 0 to 10, exported at frame 10 |
| jiggle/frame-20 | Jiggle | the same, exported at frame 20 |
| delta-mush/defaults | Delta Mush | the cube made editable, point 0 moved by (80, 80, 80), then Delta Mush added with defaults |
| correction/defaults | Correction | under a Bend (strength 90) on the cube: Correction defaults, then move one corrected point through its point list |
| ffd-grid-5 | FFD | grid points 5/5/5, the centre grid point moved by (0, 100, 0) |
| explosion-fx/frame-10 | Explosion FX | defaults, frame 10 |
| box-gate/twist-narrow-x | Twist | follow-up from brief 050: O9 cube (segments 4/4/4), Twist angle 90°, mode Within Box, size (100, 250, 400) |
| box-gate/taper-narrow-x | Taper | same cube, Taper strength 50 %, Within Box, size (100, 250, 400) |
| box-gate/bulge-narrow-z | Bulge | same cube, Bulge strength 50 %, Within Box, size (400, 250, 100) |
| box-gate/bend-rotated | Bend | same cube, Bend strength 90°, Within Box, size (100, 250, 400), deformer rotated H 45° |
| box-gate/bend-angle | Bend | same cube, Bend strength 90°, angle 45°, Within Box, size (100, 250, 400) |
| box-gate/bend-equal | Bend | same cube (200 cm), Bend strength 90°, Within Box, size (200, 250, 200) exactly the extent |
| falloff/bend-offset | Bend | follow-up from brief 061: the O24 `bend-spherical` setup with the cube moved to (0, 100, 0) (the field stays at the origin) |
| falloff/bend-strength-200 | Bend | O24 `bend-spherical` with the field's Strength 200 % and Clamp Max off |
| falloff/shear-spherical | Shear | O9 cube, Shear defaults with strength 50, a default Spherical field in its falloff list |
| falloff/spherify-linear | Spherify | O9 cube, Spherify 100 %, a default Linear field in its falloff list |
| falloff/field-child | Bend | O24 `bend-spherical`, with the Spherical field made a child of the Bend deformer |
| squash-f/top-bottom | Squash & Stretch | follow-up from brief 051: O9 cube, Top 50, Bottom −150, Factor 50 % |
| squash-f/center | Squash & Stretch | O9 cube, Factor 50 %, Center 50 % |
| squash-f/aspect | Squash & Stretch | O9 cube, Factor 50 %, Aspect 50 % |
| squash-f/smooth | Squash & Stretch | O9 cube, Factor 50 %, Smooth Start 50 %, Smooth End 50 % |
| squash-f/curvature | Squash & Stretch | O9 cube, Factor 50 %, Curvature 50 %; then `-cubic` and `-quartic` with those types |
| squash-f/custom | Squash & Stretch | O9 cube, Factor 50 %, type Custom, Power 3 |
| wrap-f/movement | Wrap | O9 cube setup of `wrap/defaults`, Movement 100 |
| wrap-f/scale-z | Wrap | same, Scale Z 50 % |
| wrap-f/tension | Wrap | same, Tension 50 % |
| wrap-f/latitude | Wrap | Spherical, Latitude 0° to 90° |
| wrap-f/longitude | Wrap | Longitude 0° to 180° |
| wrap-f/size | Wrap | Width 200, Radius 100; then `wrap-f/spherical-h200` with Spherical and Height 200 |

## Parameters (filled once per deformer)

Recorded as in [deformers-2.md](deformers-2.md): label, our key (as written in `case.toml` — `read_obj`-generated snake-case names), default, declared min/max. Vector params are shown as [x, y, z]. Cycle params show defaults as the raw stored id plus the option list in the description's order, because the automatic cycle-label mapping (cycle container lookup) gave a mismatched label on Collision (see Notes), so treat the raw id as authoritative here.

### Spline (id 1008982)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Original Spline | `original_spline` | unset link | – | – |
| Modifying Spline | `modifying_spline` | unset link | – | – |
| B-Spline Approximation | `b_spline_approximation` | 10 | 1 | 100 |
| Radius | `radius` | 30 | 0 | 1e20 (kept 999999999) |
| Use Length | `use_length` | off | 0 | 1 |
| Full Polygons | `full_polygons` | off | 0 | 1 |
| Shape | `shape` (a shape spline, SplineData) | default spline | – | our case writes it as a linked helper spline |

### Spline Rail (id 1008796)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Z Spline Left | `z_spline_left` | unset link | – | – |
| Z Spline Right | `z_spline_right` | unset link | – | – |
| X Spline Top | `x_spline_top` | unset link | – | – |
| X Spline Bottom | `x_spline_bottom` | unset link | – | – |
| Reference | `reference` | unset link | – | – |
| Mode | `mode` | 2 | – | options (description order): Limited, Within Box, Unlimited |
| Size | `size` | [200, 200, 200] | [0, 0, 0] | [1e20, 1e20, 1e20] |
| Scale Before Start | `scale_before_start` | 1.0 | ±1e18 (kept ±999999999) | |
| Scale After End | `scale_after_end` | 1.0 | ±1e18 (kept ±999999999) | |

### Mesh (cage) deformer (id 1024543)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Auto Init | `auto_init` | off | 0 | 1 |
| Memory | `memory` (read-only text) | "0 B" | – | – |
| Strength | `strength` | 1.0 | ±1e18 (kept ±999999999) | |
| Cages | `cages` (InExcludeData list) | empty | – | – |
| Accuracy | `accuracy` | 0.1 | 0 | 1 |
| External | `external` | Ignore | – | options: Ignore, Surface, Surface (Area) |

Buttons: Initialize (1001), Restore (1002). Has a Field list (440000245).

### Surface deformer (id 1024552)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Type | `type` | Projection | – | options: Projection, Mapping (U,V), Mapping (V,U) |
| Memory | `memory` (read-only text) | "0 B" | – | – |
| Strength | `strength` | 1.0 | ±1e18 (kept ±999999999) | |
| Proportional | `proportional` | off | 0 | 1 |
| Surface | `surface` | unset link | – | – |

Buttons: Initialize (1001), Restore (1002). Has a Field list.

### Camera deformer (id 1024476)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Camera | `camera` | unset link | – | – |
| Strength | `strength` | 1.0 | ±1e18 (kept ±999999999) | |
| Grid X | `grid_x` | 32 | 1 | 256 |
| Grid Y | `grid_y` | 32 | 1 | 256 |
| Safe Frame | `safe_frame` | on | 0 | 1 |
| Draw Grid | `draw_grid` | Active* | – | options: Always, Active, Hierarchy |

\* stored default raw value was 1 while the option list is ['Always', 'Active', 'Hierarchy']; the direct write test was not run on it, so the label mapping was not confirmed (same cycle caveat as Notes). Reset button (2005). Has a Field list.

### Collision deformer (id 1024544)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Falloff | `falloff` | 3 (raw) | – | options (description order): None, UV, Distance, Surface, Collider |
| Distance | `distance` | 100 | 0 | 1e20 (kept 999999999) |
| Strength | `strength_percent` | 100 % | 0 | 1e20 % |
| Restore Shape | `restore_shape_percent` | 100 % | 0 | 100 % |
| Curve | `curve` (a spline) | default curve | – | not recorded |
| Solver | `solver` | 0 (raw) | – | options (description order): Inside, Inside (Stretch), Outside, Outside (Volume), Intersect |
| Objects (solver list) | not written by read_obj (InExcludeData) | empty | – | – |
| Objects (second, 1017) | `…` | empty | – | – |
| Stretch/Relax group | size 1.0, steps 1, stretch 8, relax 4, stiffness 75 %, struct 100 %, flex 100 % | | | keys `size`, `steps`, `stretch`, `relax`, `stiffness_percent`, `struct_percent`, `flex_percent` |
| Collision group | falloff 1004 `falloff_1004`, invert 1005 `invert` | invert off | 0 | 1 |
| Falloff group | falloff 1018 `falloff_1018`, invert 1006 `invert_1006` | invert off | 0 | 1 |
| Cache section | Enable off (`enable`), Compress off (`compress`), Auto Time on (`auto_time`), Cache 20 (`cache`, min 1 max 1000), Scale 100 % (`scale_percent`) | | | Start/Stop/Offset are BaseTime, not sampled |

Buttons: Calculate (2001), Update (2010), Empty Cache (2003), Load… (2006), Save… (2007). Has a Field list.

### Jiggle (id 1021284)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| On Stopped | `on_stopped` | off | 0 | 1 |
| Local | `local` | off | 0 | 1 |
| Strength | `strength_percent` | 100 % | 0 | 1e20 % |
| Stiffness | `stiffness_percent` | 40 % | 0 | 100 % |
| Structural | `structural_percent` | 100 % | 0 | 100 % |
| Drag | `drag` | 0.1 | 0 | 1 |
| Motion Scale | `motion_scale` | 1.0 | 0 | 1e20 (kept 999999999) |
| Springs | `springs` | 4 | 0 | 100 |
| Iterations | `iterations` | 4 | 1 | 1000 |
| Restrict | (InExcludeData) | empty | – | – |
| Gravity | `gravity` | 0 | ±1e20 | |
| Forces | (InExcludeData) | empty | – | – |
| Cache section | Enable off, Compress off, Auto Time on, Cache 20, Scale 100 % | | | as Collision above |
| several "Map" link fields | not recorded (unset vertex maps) | | | |

Buttons: Calculate (2014), Update (2026), Empty Cache (2016), Load… (2019), Save… (2020). Has a Field list.

### Delta Mush (id 1035447)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Strength | `strength` | 1.0 | ±1e18 (kept ±999999999) | |
| Iterations | `iterations` | 10 | 0 | 2147483647 (kept 999999999) |
| Smooth View | `smooth_view` | 0.0 | ±1e18 | |
| Tangent Mode | `tangent_mode` | Edge | – | options: Edge, Neighbour Points, Polygon, Based On UVs |
| Smooth Type | `smooth_type` | 1 | – | options (description order): Fixed, Proportional |
| Pin Border Points | `pin_border_points` | off | 0 | 1 |
| Mode | `mode` | 1 | – | options (description order): Exclude, Include |
| Tags | `tags` (InExcludeData) | empty | – | – |

Buttons: Initialize (1005), Release (1006). Has a Field list.

### Correction (id 1024542)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Lock | `lock` | off | 0 | 1 |
| Scale | `scale` | off | 0 | 1 |
| Mapping | `mapping` | 1 | – | options (description order): UV, Nearest, Normals |
| Strength | `strength` | 1.0 | ±1e18 (kept ±999999999) | |

Buttons: Update (1003), Freeze (1004), Reset (1005). Has a Field list. The corrected points are not in the Attribute description; they are edited through the object's own point list (the case moved corrected point 0 by (50, 0, 0)).

### Explosion FX (id 1002603)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Time | `time_percent` | 10 % | 0 | 100 % |
| CL Thickness | `cl_thickness` | 10 | ±1e20 (kept ±999999999) | |
| CL Thickness Variation | `cl_thickness_variation_percent` | 0 % | 0 | 100 % |
| CL Density | `cl_density` | 1000 | 0 | 1e20 |
| CL Density Variation | `cl_density_variation_percent` | 0 % | 0 | 100 % |
| CL Cluster Type | `cl_cluster_type` | Selections + Polys | – | options (description order): Polygons, Automatic, Use Selection Tags, Selections + Polys |
| CL Mask | `cl_mask` (text) | "" | – | – |
| CL Fix Unselected | `cl_fix_unselected` | on | 0 | 1 |
| CL Min Polys | `cl_min_polys` | 5 | 1 | 2147483647 |
| CL Max Polys | `cl_max_polys` | 10 | 2 | 2147483647 |
| CL Disappear | `cl_disappear` | off | 0 | 1 |
| CL Disappear Kind | `cl_disappear_kind` | Time | – | options: Time, Distance |
| CL Disappear Start | `cl_disappear_start` | 10 | 0 | 1e20 |
| CL Disappear Duration | `cl_disappear_duration` | 10 | 0 | 1e20 |
| EX Strength | `ex_strength` | 1000 | 0 | 1e20 |
| EX Strength Decay | `ex_strength_decay_percent` | 0 % | 0 | 100 % |
| EX Strength Variation | `ex_strength_variation_percent` | 0 % | 0 | 100 % |
| EX Direction | `ex_direction` | All | – | options (description order): All, Only X, Except X, Only Y, Except Y, Only Z, Except Z |
| EX Direction Linear | `ex_direction_linear` | off | 0 | 1 |
| EX Direction Variation | `ex_direction_variation_percent` | 0 % | 0 | 100 % |
| EX Blast Time | `ex_blast_time` | 1000 | 0 | 1e20 |
| EX Blast Speed | `ex_blast_speed` | 50 | 0 | 1e20 |
| EX Blast Speed Decay | `ex_blast_speed_decay_percent` | 0 % | 0 | 100 % |
| EX Blast Speed Variation | `ex_blast_speed_variation_percent` | 0 % | 0 | 100 % |
| EX Blast Range | `ex_blast_range` | 200 | 0 | 1e20 |
| EX Blast Range Variation | `ex_blast_range_variation_percent` | 0 % | 0 | 100 % |
| GR Acceleration | `gr_acceleration` | 9.81 | 0 | 1e20 |
| GR Acceleration Variation | `gr_acceleration_variation_percent` | 0 % | 0 | 100 % |
| GR Direction | `gr_direction` | -Y World | – | options (description order): None, X World, -X World, Y World, -Y World, Z World, -Z World |
| GR Range | `gr_range` | 300 | 0 | 1e20 |
| GR Range Variation | `gr_range_variation_percent` | 0 % | 0 | 100 % |
| RT Speed | `rt_speed` | 0 | ±1e20 | |
| RT Speed Decay | `rt_speed_decay_percent` | 0 % | 0 | 100 % |
| RT Speed Variation | `rt_speed_variation_percent` | 0 % | 0 | 100 % |
| RT Rotation Axis | `rt_rotation_axis` | Center Of Gravity | – | options: Center Of Gravity, X-Axis, Y-Axis, Z-Axis |
| RT Rotation Axis Variation | `rt_rotation_axis_variation_percent` | 0 % | 0 | 100 % |
| SP Wind | `sp_wind` | 0 | ±1e20 | |
| SP Wind Variation | `sp_wind_variation_percent` | 0 % | 0 | 100 % |
| SP Twist | `sp_twist` | 0 | ±1e20 | |
| SP Twist Variation | `sp_twist_variation_percent` | 0 % | 0 | 100 % |

No Field list on Explosion FX.

## Findings (scripted evaluation)

- **Mesh deformer and Camera deformer produced no deformation in the exported meshes** under
  scripted/background evaluation on this build, for every workaround tried (Initialize/Restore
  buttons, Auto Init on, SetDirty, moving the grid point, linking a camera into the view's
  BASEDRAW_DATA_CAMERA). GetDeformCache() returns nothing for any deformer under ExecutePasses here
  (even a plain Bend), while SaveDocument's own evaluation does deform Bend etc., so the two cases
  are recorded as observed (camera-deformer/grid exported the undeformed cube; mesh-deformer/cage
  exported the cube undeformed plus the edited cage object as `Cube.1`). These meshes should be
  regenerated interactively if deformation is required for parity.
- Helper polygon objects leak into the scene-wide OBJ export, matching the O3 caps precedent: the
  cage in mesh-deformer/cage (object `Cube.1`), the surface in surface/defaults (`Plane`), and the
  collider in collision/sphere (`Sphere`). The Spline/Rail/Helix helper splines are not polygons and
  do not appear. The FFD and Camera deformer gizmos are not exported as polygons.
- The automatic cycle-label mapping (resolving enum ids through the DESC_CYCLE container) gave a
  label mismatch on Collision: the raw stored Solver value read directly is 0, whose description
  order suggests `Inside`, while the cycle-container lookup resolved it to `Intersect`, and the
  Falloff value 3 resolved to `None` (order suggests `Surface`). Raw ids and option lists are
  recorded above; treat id→label for these two fields as unresolved. This mapping has worked
  correctly on the classical deformers in earlier jobs, so only Collision's enums are in doubt.
- jiggle/frame-10 and /frame-20 exported the cube rigidly translated to x 300 (vertices unchanged,
  bbox exactly the moved cube): no jiggle lag or vertex deformation appeared in the export. Jiggle
  deformation depends on its solver cache, which the scripted path does not build here.
- delta-mush/defaults: exactly two point coordinates differ from the plain cube grid (the moved
  point 0 and one neighbour); no visible smoothing spread in the export. Same solver-cache caveat.
- collision/sphere: the cube is undeformed in the export (all 162 cube coordinates identical to the
  plain cube grid); only the sphere helper geometry is added (114 more vertices).
- falloff/shear-spherical exported geometry identical to the plain cube grid: with the default
  spherical field (size 200 at the origin) the shear shift, which is proportional to y, is negligible
  inside the field's reach. The field weighting itself is not independently confirmed by this mesh.
- In falloff/bend-offset the field stays at the origin (0, 0, 0) while the cube moves to (0, 100, 0)
  — the row's parenthetical is taken literally (reading it the other way, field at the cube, would
  need re-observation if brief 061 meant that).
- correction/defaults: which corrected point to move is not specified by the row; corrected point 0
  moved by (50, 0, 0).

## Notes

Folders: every family in this job keeps its folder named after the row's first component,
top-level under `tests/golden/` (`spline-deformer`, `spline-rail`, `mesh-deformer`, `surface`,
`camera-deformer`, `collision`, `jiggle`, `delta-mush`, `correction`, `explosion-fx`, `box-gate`,
`falloff`, `squash-f`, `wrap-f`). The `ffd/ffd-grid-5` case is nested under the `ffd` folder that
job O9 created, next to that job's cases.

Per-case result overview (vertex/face counts and bounds from each case's `mesh.obj`):

| Case | verts/faces | bbox min | bbox max |
|---|---|---|---|
| spline-deformer/defaults | 162/160 | (-100, -200, -200) | (100, 200, 100) |
| spline-rail/defaults | 162/160 | (-350, -200.25, -199.98) | (350, 200, 0) |
| mesh-deformer/cage | 170/166 | (-250, -250, -150) | (150, 250, 150) |
| surface/defaults | 283/260 (cube + plane) | (-100, -200, -200) | (224.6484, 467.3492, 200) |
| camera-deformer/grid | 162/160 | (-100, -200, -100) | (100, 200, 100) |
| collision/sphere | 276/288 (cube + sphere) | (-100, -200, -100) | (250, 200, 100) |
| jiggle/frame-10 | 162/160 | (200, -200, -100) | (400, 200, 100) |
| jiggle/frame-20 | 162/160 | (200, -200, -100) | (400, 200, 100) |
| delta-mush/defaults | 162/160 | (-100, -200, -100) | (100, 200, 100) |
| correction/defaults | 162/160 | (-100, -200, -100) | (359.154943, 259.154943, 100) |
| ffd/ffd-grid-5 | 162/160 | (-100, -200, -100) | (100, 200, 100) |
| explosion-fx/frame-10 | 672/618 | (-177.731545, -237.659725, -148.18575) | (118.167887, 197.497872, 129.770756) |
| box-gate/twist-narrow-x | 98/96 | (-111.800177, -100, -111.800177) | (111.800177, 100, 111.800177) |
| box-gate/taper-narrow-x | 98/96 | (-100, -100, -100) | (100, 100, 100) |
| box-gate/bulge-narrow-z | 98/96 | (-150, -100, -100) | (150, 100, 100) |
| box-gate/bend-rotated | 98/96 | (-100, -108.810876, -141.553768) | (190.26559, 141.553768, 108.810876) |
| box-gate/bend-angle | 98/96 | (-100, -100, -100) | (240.584979, 260.416693, 248.406702) |
| box-gate/bend-equal | 98/96 | (-95.578207, -100.102682, -100) | (234.257625, 229.73315, 100) |
| falloff/bend-offset | 324/320 | (-100, -200, -100) | (191.000027, 300, 100) |
| falloff/bend-strength-200 | 162/160 | (-100, -200, -100) | (359.154943, 259.154943, 100) |
| falloff/shear-spherical | 162/160 | (-100, -200, -100) | (100, 200, 100) |
| falloff/spherify-linear | 162/160 | (-100, -200, -159.164079) | (200, 200, 159.164079) |
| falloff/field-child | 162/160 | (-100, -200, -100) | (359.154943, 259.154943, 100) |
| squash-f/top-bottom | 162/160 | (-200, -124.9975, -200) | (200, 174.9975, 200) |
| squash-f/center | 162/160 | (-200, -149.9975, -200) | (200, 149.9975, 200) |
| squash-f/aspect | 162/160 | (-150, -149.9975, -200) | (150, 149.9975, 200) |
| squash-f/smooth | 162/160 | (-200, -149.9975, -200) | (200, 149.9975, 200) |
| squash-f/curvature | 162/160 | (-200, -149.9975, -200) | (200, 149.9975, 200) |
| squash-f/curvature-cubic | 162/160 | (-200, -149.9975, -200) | (200, 149.9975, 200) |
| squash-f/curvature-quartic | 162/160 | (-200, -149.9975, -200) | (200, 149.9975, 200) |
| squash-f/custom | 162/160 | (-200, -149.9975, -200) | (200, 149.9975, 200) |
| wrap-f/movement | 162/160 | (-212.132034, -175, -100) | (212.132034, 275, 129.289322) |
| wrap-f/scale-z | 162/160 | (-176.776695, -200, -50) | (176.776695, 200, 93.933983) |
| wrap-f/tension | 162/160 | (-184.605834, -200, -100) | (184.605834, 200, 114.644661) |
| wrap-f/latitude | 162/160 | (-212.132034, -300, -100) | (212.132034, 0, 200) |
| wrap-f/longitude | 162/160 | (-212.132034, -200, 270.710678) | (212.132034, 200, 500) |
| wrap-f/size | 162/160 | (-200, -200, -100) | (200, 200, 100) (wrapped: point radii 0–223.61) |
| wrap-f/spherical-h200 | 162/160 | (-212.132034, -300, -100) | (212.132034, 300, 200) |

The three derived cases the queue flagged (squash-f/curvature-cubic, squash-f/curvature-quartic,
wrap-f/spherical-h200) were observed as additional cases, as the rows request.
