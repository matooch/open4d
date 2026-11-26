# SCENE-OBJECTS — Camera, Light, Environment, Floor, Sky and stored render settings (brief 047)

Sources: the observation `docs/parity/scene-defaults.md` (O29) sections 2 (camera), 3 (light),
4 (the material's Environment group, for the environment defaults) and 5 (render settings of a
new document); `docs/parity/camera.md` (O57) for the camera readbacks; our own design where O29
observed nothing (Environment, Floor and Sky objects, see S4). Rendering rules are in
[RENDER.md](RENDER.md) (R8, R10, R11, R12).

Conventions: parameter units are stored as the engine uses them: angles in radians, percentages as
fractions (100 % = 1.0), distances in scene units (the O29 tables are in centimetres, the same
number). Every node of these types is `Category::Object` with the usual transform parameters, no
geometry, and appears in the object tree. Scale and shear of the node are ignored: cameras and
lights use the rigid part of the world matrix.

## S1 Node types

| Type id | Label | Source |
|---|---|---|
| `forge.camera` | Camera | O29 section 2 (Object, Physical, Details tabs), O57 |
| `forge.light` | Light | O29 section 3 (General, Details, Shadow tabs) |
| `forge.environment` | Environment | design; defaults from O29 section 4 (Environment group) |
| `forge.floor` | Floor | design |
| `forge.sky` | Sky | design |

The Create menu has a **Scene** submenu with the five (`forge_ui::commands::scene_menu`); the types
are not repeated under Objects. Icons: `camera`, `light`, `environment`, `floor`, `sky`
(`crates/ui/icons`), registered in `icon_table.rs`; every command has one too (`render.settings`
uses `settings`, `view.look_through_camera` uses `camera`).

## S2 Camera (`forge.camera`)

Parameter ids are the O29 "our key" except where noted; defaults, ranges and options are O29
section 2 and the test `camera_defaults_equal_o29` reads the observation table and compares every
default.

| Group | Id (O29 key) | Kind, default, range |
|---|---|---|
| Object | `projection` | enum `perspective` (default), `parallel`, `left`, `right`, `front`, `back`, `top`, `bottom`, `military`, `frog`, `bird`, `gentleman`, `isometric`, `dimetric` |
| Object | `sensor_size` (`sensor_size_film_gate`) | float 36, 1..2000 |
| Object | `fov_horizontal` (`field_of_view_horizontal`) | angle 53.13010235415599 deg (`2 atan(1/2)`), 0.2..174 deg |
| Object | `zoom` | float 1, 0.0001..1e6 |
| Object | `film_offset_x`, `film_offset_y` | percent 0 |
| Object | `focus_distance` | distance 2000, 0..999999999 |
| Object | `use_target_object`, `focus_object` | bool false, link (object) |
| Object | `white_balance_k` | float 6500, 1000..10000 |
| Object | `affect_lights_only`, `export_to_compositing` | bool false, true |
| Physical | `movie_camera`, `exposure`, `diaphragm_shape` | bool false |
| Physical | `f_stop` (`f_stop_f`) | float 8, 0.01..256 |
| Physical | `iso`, `gain_db` | float 200 (10..10000), 0 (-100..100) |
| Physical | `shutter_speed` (`shutter_speed_s`) | float 1/30 s, 0.0001..100 |
| Physical | `shutter_angle`, `shutter_offset` | angle 180 deg (10..1080), 0 deg (0..360) |
| Physical | `shutter_efficiency` | percent 70 (0..100) |
| Physical | `lens_distortion_quadratic`, `lens_distortion_cubic` | percent 0 (-100..100) |
| Physical | `vignetting_intensity`, `vignetting_offset` | percent 0 (0..500, 0..100) |
| Physical | `chromatic_aberration` | percent 0 (-1000..1000) |
| Physical | `blades`, `diaphragm_angle` (`angle`), `bias`, `anisotropy` | int 6 (3..32), angle 0, float 0 (-1..1), percent 0 (-100..100) |
| Details | `enable_near_clipping`, `near_clipping` | bool true, distance 0 |
| Details | `enable_far_clipping`, `far_clipping` | bool false, distance 10000 |
| Details | `show_cone` | bool true (draw the frustum in the viewport) |

Not parameters yet (O29 lists them; no renderer or viewport rule exists): Physical `shader`,
the DOF map (front/rear blur), the Stereoscopic, Composition Helpers and Spherical Camera tabs.
They are listed here so the next brief adds them with their O29 rows.

**Lens rules.** The focal length has no field of its own: the horizontal field of view drives
it (O57 notes: writing the FOV changes the render and the displayed focal length, writing Zoom
changes neither). `focal_mm = sensor / (2 tan(fov/2))` (FOV 53.13 deg reads 36 mm, 90 deg 18 mm,
20.4 deg 100 mm, O57). The vertical FOV is a readback: `2 atan(tan(fov/2) / aspect)` with the
film aspect of the render settings (O29: 1280 x 720 reads 31.41727565803149 deg; test
`fov_readbacks_match_o29`). The film offset X is a fraction of the image width, Y of the image height. **Sign (O57 `film-offset`,
`film-offset-y`)**: the parameters shift the frame, so a positive X moves the picture left and a positive Y moves
it up; `CameraData` and the snapshot camera carry the principal-point shift: minus X, plus Y (RENDER.md R15).
The thin lens of the render has radius `focal / (2 f-stop)` in millimetres converted to scene
units (x 0.1) and the `focus_distance` (RENDER.md R15); it is used when the render settings turn
depth of field on.

**Projections.** `perspective` uses the lens above. The axis views `front`, `back`, `left`,
`right`, `top`, `bottom` are orthographic and **ignore the node's rotation**: the camera looks
along +Z (front), -Z (back), +X (left), -X (right), -Y (top, screen up = +Z) or +Y (bottom,
screen up = -Z) from the node's position (O57 `ortho-top`: the case file records a rotation and
the picture looks straight down; `ortho-front`, `ortho-top` fix Front and Top, the other four
follow by symmetry and are **parity-pending**; brief 138 matches both exactly, RENDER.md R15). `parallel` is orthographic along the node's own
axes. `military`, `frog`, `bird`, `gentleman`, `isometric`, `dimetric` are not modelled: they
render as `parallel` and the render reports a warning. The orthographic width at zoom 1 is
1000 units (O57 `ortho-front`: exact at zero tolerance, RENDER.md R15); other zoom values are
**parity-pending** (`width = 1000 / zoom`).

**Rotation convention (finding).** The reference's camera heading sign is opposite the usual
right-handed one: the node rotation is `Ry(-h) Rx(-p) Rz(-b)` (TRANSFORM.md), so a heading of
-15 degrees turns the camera to the right (O30) and a positive pitch looks up. That matrix is
exactly the render API's `hpb_matrix` (RENDER.md R10); camera objects take it unchanged from the
node's world matrix (test `heading_sign_is_the_references_not_the_right_handed_one`).

## S3 Light (`forge.light`)

All nine Type options share one parameter set in O29 (only the visibility of some controls
depends on the type), so there is one node type with `type` = `omni` (default), `spot`,
`infinite`, `area`, `square_spot`, `parallel`, `parallel_spot`, `square_parallel_spot`, `ies`.
The test `light_defaults_equal_o29_for_every_type` compares every default against the table of
each of the nine types.

General: `color` white, `use_temperature` false, `color_temperature` 6500 (1000..10000),
`intensity` 100 % (-1e7 %..1e7 %), `type`, `shadow` (`none`, `shadow_maps`, `raytraced`, `area`;
default none), `visible_light` (`none`, `visible`, `volumetric`, `inverse_volumetric`),
`no_illumination` false, `ambient_illumination` false, `diffuse` true, `specular` true,
`separate_pass` false, `gi_illumination` true, `export_to_compositing` true.
Details: `use_inner_value` true, `inner_angle` 0 and `outer_angle` 30 deg (0..175), `aspect_ratio`
1, `contrast` 0 %, `shadow_caster` false, `area_shape` (`disc`, `rectangle` default, `line`,
`sphere`, `cylinder`, `cylinder_perpendicular`, `cube`, `hemisphere`, `object`), `object` link,
`size_x`/`size_y`/`size_z` 200, `falloff_angle` 180 deg, `infinite_angle` 0.5 deg, `samples` 40,
`show_in_render` false, `show_as_solid_in_viewport` false, `show_in_specular` true,
`show_in_reflection` false, `falloff` (`none` default, `inverse_square`, `linear`, `step`,
`inverse_square_clamped`), `falloff_inner_radius` 0, `falloff_radius` (`falloff_radius_decay`) 500,
`colored_edge_falloff`, `z_direction_only`, near/far clipping (`use_near_clipping` false from 0
to 10, `use_far_clipping` false from 90 to 100). Shadow: `shadow_density` 100 %, `shadow_color`
black. Not parameters yet: Visibility (volumetric), Photometric, Caustics, Noise, Lens Effects,
the Project include list, the gradient colour and the hidden shadow-map controls.

**Mapping to the render API** (`forge_render_bridge::light_from_data`, RENDER.md R8):

| Light | Render light |
|---|---|
| omni | `Omni` |
| spot | `Spot { inner, outer }` (full cone angles, as R8; inner = 0 when `use_inner_value` is off) |
| infinite, parallel | `Infinite { angle: infinite_angle }` |
| area | `Area` with `disc` (radius `size_x / 2`), `rectangle` (`size_x` x `size_y`), `sphere` (radius `size_x / 2`); **Unsupported** shapes (`line`, `cylinder`, `cylinder_perpendicular`, `cube`, `hemisphere`, `object`): the light is skipped with a warning |
| square_spot | round `Spot` + warning (the square cone is not modelled) |
| parallel_spot, square_parallel_spot | `Infinite` + warning (the cone is not modelled) |
| ies | `Omni` + warning (the profile is not read) |

`color` times `kelvin_tint(color_temperature)` when `use_temperature` (a public polynomial fit of
the Planckian locus normalised to white at 6500 K; the reference's own temperature curve is
not observed, **parity-pending**). `intensity` is the fraction. `falloff`: `inverse_square` and
`inverse_square_clamped` map to `InverseSquare` (R8's is clamped), `linear` to `Linear`, `step` to
`None` with a warning. `shadow`: `raytraced` = `Hard`, `shadow_maps` and `area` = `Soft`
(radius 0 until the soft-shadow size is observed, **parity-pending**). `no_illumination` skips
the light; `diffuse`, `specular` are passed; `show_in_render` is `visible`.

## S4 Environment, Sky, Floor (design, not observed)

O29 observed no such objects; these are ours and carry no parity claim.

- `forge.environment`: `color` white and `brightness` 100 % (the defaults of the material's
  Environment group, O29 section 4), `visible_in_background` false, `image` (HDRI file path,
  empty = uniform colour; Radiance HDR, OpenEXR or an LDR file, lat-long, loaded by the render
  bridge (`load_environment_image`, the scene loader resolves a relative path against the scene
  file); an unreadable file warns and keeps the uniform colour), `image_rotation` 0. Lighting: `Lighting::Constant { color, intensity: brightness }`, or with an image
  `Lighting::Image { image, rotation: image_rotation, intensity: brightness }` (the colour
  applies to the uniform case only), importance sampled by the CPU tracer (RENDER.md R16).
- `forge.sky`: `zenith_color`, `horizon_color`, `ground_color`, `brightness` 100 %,
  `visible_in_background` true. Lighting: `Lighting::Sky` (R8).
- `forge.floor`: `color` 0.8 grey (the default object colour of O29 section 1), `extent`
  100000. The local XZ plane, +Y up, as two triangles of half size `extent`, with a flat
  Standard material of `color`.

With both an Environment and a Sky the first Environment is used and the render warns. With a
background flag set the miss colour is `Background::Environment`.

## S5 Scene objects in the render (`forge_render_bridge::apply_scene_objects`)

Objects hidden for rendering (visibility, layer) are not part of the render.
- Camera: the **first** camera in tree order replaces the framing camera of the adapter
  (`snapshot_from_polys`); the render warns when there are several. Without a camera the
  framing camera is unchanged.
- Lights: each light with illumination on becomes a snapshot light. If the scene has any light
  object, even switched off, the default light (R11) is off: `Applied::scene_has_lights`, which
  `forge render` turns into `default_light = false`. Without light objects the default light
  stays.
- Environment/Sky/Floor as in S4. Warnings are collected in `Applied::warnings`; `forge render`
  prints them to stderr and returns them in its `--json` result (`warnings`).

## S6 Render settings in the scene file

`forge_core::RenderConfig { width, height, samples, max_depth, seed, default_light, filter,
filter_width }` with the O29 section 5 defaults of a new document (1280 x 720, Cubic filter of
width 2; samples 16, depth 5, seed 1 follow R2). The scene holds `Option<RenderConfig>`:
`None` (the default) means the document stores none and the caller's own defaults apply
(`forge render`: 320 x 240, 16 samples, Box filter, as before).

**File format: no version bump.** The field `render` of the scene file is a pure addition to
version 12: `#[serde(default, skip_serializing_if = "Option::is_none")]`, so a scene without
settings writes the same bytes as before, an old file loads with none, and an invalid stored
configuration is dropped with a warning on load. Edits go through `Tx::set_render_config`
(undoable). Version 13 is only needed when a later brief adds a field that old readers must not
ignore.

Commands: `render.settings` (optional `width`, `height`, `samples`, `max_depth`, `seed`,
`default_light`, `filter`, `filter_width`, or `reset: true`; undoable and merged while editing).
`forge render` uses the stored settings as defaults and its flags (`--width`, ...) override them
(`forge_render_bridge::settings_from_scene`).

## S7 Viewport

- `forge_viewport::scene_view::look_through(world, lens, aspect, distance)`: the viewport camera
  that sees what the camera object sees. It projects points to the same pixels as the render
  API's formula (`px = w/2 + f_px x/z`, test `the_viewport_camera_projects_like_the_formula`,
  several lenses, headings and pitches). Film offset and bank are not representable by the
  orbit camera and are dropped. The command `view.look_through_camera` (`node` default: the
  selected camera, else the first one; `aspect` default: the render settings, else 16:9) sets
  it through `view.set_camera`'s machinery; the editor viewport is left-handed (Z mirrored).
- Gizmos (`camera_lines`, `light_lines`, `floor_lines`) are line lists in world space drawn by
  the scene overlay (`SceneOverlay::objects`): the camera frustum 100 units deep with an up
  marker (when `show_cone`), the light shapes (omni: three circles; spot: a cone; infinite: a
  circle with rays; area: its rectangle, disc or sphere with a normal tick) and a 10 x 10
  grid of the floor (half size min(extent, 1000)). Selected objects are drawn orange.

## S8 Not done

Stereoscopic, composition helpers, spherical camera, the DOF map and the physical camera's
effects (motion blur, lens distortion, vignetting, bokeh shape; brief 138 added projection
parity and a thin-lens depth of field, RENDER.md R15), photometric and volumetric light parameters, object-shaped area lights, an explicit "render camera" selection (the first camera is used), per-light shadow
softness radius.

## S9 Null display (`forge.null`, brief 046)

This file documents the scene helper objects drawn only in the viewport (camera, light, floor),
so the Null's display options live here. Source: the observation
[generators-3.md](../parity/generators-3.md), section "O20b observation" (Null display shapes,
read through the bridge's `viewport_screenshot`, scale 0.785 px per unit, extents include about
1-2 units of line width and a calibration bias of about 1.9 units on the centre). The Null
still has no geometry: it contributes nothing to renders or exports (`null_never_contributes`),
and a Null parent passes its children on as before (CORE.md section 4).

### S9.1 Parameters

Declared on the node type, so the Attribute panel, undo, animation and files derive from them. A
file saved before this brief has no such params and loads with the defaults (CORE.md, "Missing
params on known types are filled with defaults"); the file version does not change.

| Id | Label | Kind, default | Range / options | Source |
|---|---|---|---|---|
| `shape` | Display | enum `dot` | `none, dot, locator, circle, rectangle, diamond, triangle, pentagon, hexagon, octagon, star, axis, cube, pyramid, sphere` | O20b parameter table (default Dot) |
| `radius` | Radius | distance 10 | 0..1e20 | O20b (slider 0-100, step 1 is UI only) |
| `aspect` | Aspect Ratio | float 1 | 0..1e20 | O20b (step 0.01) |
| `orientation` | Orientation | enum `camera` | `camera, x, y, z` | O20b |
| `display_color` | Display Color | enum `material` | `material, layer, automatic, custom` | O20b |
| `color` | Color | color (1, 1, 1) | 0..1 per channel | O20b, the Color after Display Color |
| `icon_color` | Icon Color | color (1, 0.9, 0.4) | 0..1 per channel | O20b, the Color after Icon Color; stored only, the object tree icon tint is not drawn (deferred) |
| `xray` | X-Ray | bool false | | O20b; stored, **not drawn yet** (deferred, see S9.5) |

Visible in Editor / Visible in Renderer (Default, On, Off) are the generic `object.*`
visibility params every object has (CORE.md section 7); the Null needs nothing extra.

### S9.2 The drawing frame

Every shape is drawn centred on the Null in a plane with an in-plane horizontal axis `u`, a
vertical axis `v` and a normal `n`. R is Radius, A is Aspect Ratio.

| Orientation | u | v | n | Proof |
|---|---|---|---|---|
| `camera` | viewport camera right | viewport camera up | toward the viewer | Circle in Front and Top view is a full circle (diameter 201.3 = 2R + line width): it follows the view |
| `x` | world Z | world Y | world X | circle is a vertical line in Front view (2.5 x 198.7), also vertical in Top: plane YZ |
| `y` | world X | world Z | world Y | horizontal line in Front (198.7 x 2.5), a circle in Top: plane XZ |
| `z` | world X | world Y | world Z | circle in Front, horizontal line in Top (198.7 x 1.3): plane XY |

Which of the two in-plane axes is `v` for the X and Y rows is **own design, parity-pending O20b**
(the circle used for the rows is symmetric): `v` is world Y for `x`, world Z for `y`, so a vertical
shape stays up when the plane contains the up axis. Aspect multiplies `v` (below).

A Null with a fixed orientation (`x`, `y`, `z`) is drawn in its local frame, so its world matrix
(rotation, scale, position) moves the drawing (own design, parity-pending O20b: the observation
used an untransformed Null). `camera` orientation is drawn at the Null's world position facing the
viewport camera; rotation and scale of the Null do not turn it, and R is used as is.

Aspect: **2D shapes** multiply the `v` coordinate by A (`Rectangle R50 AR2`: X 71.3 stays
50 sqrt(2) = 70.7 + line width, Y 142.7 = 2 x 70.7 + line width). **3D shapes (cube, pyramid,
sphere) ignore Aspect** (`Pyramid R50 AR2` was byte-identical to AR1 in Front and Top; a possible
effect along the depth axis was not observable; own design, parity-pending O20b: ignoring it is
the simplest rule that agrees with the one observation).

### S9.3 Shapes, with points as (u, v) before the Aspect factor

Segment counts are those observed; every closed outline is a loop of that many segments.

| Shape | Drawn geometry | Proving measurement (R = 100) |
|---|---|---|
| `none` | nothing | no pixels beyond the Shape = None baseline |
| `dot` | one point at the origin, drawn as a small marker (not a segment) | about 2.5 x 2.5 units (3-4 px), 1 point |
| `locator` | two lines: (-R, 0)-(R, 0) and (0, -R)-(0, R) | 198.7 x 198.7, 2 lines |
| `circle` | closed circle of radius R, **64 segments** (own design, parity-pending O20b: the count was not read; 64 is smooth up to the largest sizes without cost) | 201.3 x 201.3 (2R + line width), not filled |
| `rectangle` | 4 corners (+-R/sqrt 2, +-R/sqrt 2): a square with its corners on the circle of radius R, side R sqrt 2 | 142.7 x 142.7 = 141.4 + line width |
| `diamond` | vertices (R, 0), (0, R), (-R, 0), (0, -R) (rectangle turned 45 deg) | 198.7 x 198.7, 4 segments |
| `triangle` | regular, circumradius R, vertices at 90, 210, 330 deg: (0, R), (-R sqrt3/2, -R/2), (R sqrt3/2, -R/2): vertex up, flat base | width 173.2 = R sqrt 3, y -50.3..98.7 (base at -R/2, apex R) |
| `pentagon` | regular, circumradius R, vertices at 90 + 72k deg: vertex up, flat base | width 191.1 (2 R sin 72 = 190.2 + line), y -80.9..98.7 (base -R cos 36) |
| `hexagon` | regular, circumradius R, vertices at 60k deg: left/right vertices, flat top and bottom | width 198.7, height 173.2 = R sqrt 3 |
| `octagon` | regular, circumradius R, vertices at 22.5 + 45k deg: flat sides on the axes | 186.0 = 2 R cos 22.5 (184.8) + line width; vertices on the axes would give 198.7 |
| `star` | 16 vertices alternating outer radius R at 90 + 45k deg and inner radius r at 112.5 + 45k deg, 16 segments (8 points). The outer tips fix the extent; **inner ratio r = R/2 is own design, parity-pending O20b** (not measurable from the extent) | 198.7 x 198.7, 16 segments |
| `axis` | two open lines from the origin: (0, 0)-(R, 0) and (0, 0)-(0, R); nothing along -u or -v, no arrowheads | x 0..100.6, y -0.6..98.7, no changed pixels on -X or -Y |
| `cube` | 12 edges of the cube of half side R in the frame (u, v, n), **depth 2R (own design, parity-pending O20b)** | front view: a square of side 2R (201.3) with no inner lines (the back edges coincide) |
| `pyramid` | square base of half side R at n = -R (4 edges) and 4 edges from its corners to the apex (0, 0, +R): **8 segments**; apex toward the viewer on the normal (own design for the depth and apex direction, parity-pending O20b) | front view: the square (201.3) plus both diagonals meeting at the centre, 8 segments |
| `sphere` | three great circles of radius R, 64 segments each, in the planes (u, v), (u, n) and (v, n) (own design: three circles is the simplest solid that gives the observed view, parity-pending O20b) | front view: a circle (201.3) plus a horizontal and a vertical diameter crossing at the centre |

For the 3D shapes with orientation `camera` the frame is the **world axes** (u = X, v = Y,
n = Z) transformed by the Null's matrix, not the camera's: a cube turning with the view would always
look like a flat square (own design, parity-pending O20b; Front view agrees with the observation).

### S9.4 Colour

Lines are drawn in one colour: `material` (no material on a Null) and `automatic` use the
object gizmo grey (`OBJECT_COLOR`), `layer` the object's layer colour (grey when it has none),
`custom` the `color` param. A selected Null is drawn in the selection colour as the other
gizmos are. Hidden objects (editor visibility, layer) draw nothing.

### S9.5 Code and tests

`forge_generators::null_display` holds the pure functions `null_display_lines(shape, radius,
aspect, orientation, view) -> Vec<[DVec3; 2]>` and `null_display_points` (the Dot), the
`NullShape` / `NullOrientation` enums and `null_display_data(scene, id)`. The scene overlay
(`forge_ui::scene_overlay`) turns them into `DrawLine`s with the viewport camera's
right/up/forward. Tests: `crates/generators/tests/null_display.rs` (extents, segment counts, centring,
Aspect, Orientation, defaults, file round trip, an old file, no geometry from `evaluate`),
`crates/io/tests/null_display.rs` (nothing exported), `crates/ui/tests/null_overlay.rs` (the overlay lines,
colours, camera facing, visibility).

Deferred: **X-Ray** (drawing through objects; the overlay line pipeline is depth tested and has
no per-line flag) and the object tree icon tint from `icon_color`.
