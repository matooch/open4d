# Phase 3 — viewport (`crates/viewport`, package `forge-viewport`)

Sources: published literature and public API docs only; no reference-app observation.
- T. Möller, B. Trumbore, "Fast, Minimum Storage Ray/Triangle Intersection", JGT 2(1)
  (1997) — picking.
- C. Ericson, *Real-Time Collision Detection* (2005), §5.1 (closest points line/line),
  §5.3 (ray/plane) — gizmo drags and snapping.
- D. Eberly, "Triangulation by Ear Clipping" (Geometric Tools, 2002) — n-gon
  triangulation (in forge-geometry, GEOMETRY.md §6).
- Lambert diffuse shading with a camera headlight; wgpu 30 public API docs; glam
  `look_at_rh` / `perspective_rh` / `orthographic_rh` (depth range [0, 1]).

**Engine convention** (not a parity claim): right-handed, +Y up, as in glam. The
camera can display a left-handed world un-mirrored (`Handedness::Left`,
docs/specs/SMOOTHING.md §4). Mapping
the reference app's conventions happens at import (harness `[import]`) and needs
docs/parity/transform.md. **Deferred until that observation exists:** rotate gizmo,
object-local gizmo axes, object transforms in the viewport. UX choices that a
reference observation may change (navigation speeds, selection modes, gizmo sizes,
colours) are marked **parity-pending**; mouse/key bindings belong to Phase 4 (ui).

All math is f64 on the CPU; the GPU receives f32.

## 1. Camera (`camera`)

Orbit camera `Camera { target, distance, yaw, pitch, projection }` with
`Projection::Perspective { fov_y }` (radians) or `Projection::Orthographic { height }`
(world units visible vertically). `Camera::default()`: target 0, distance 10,
yaw 0, pitch 0, perspective fov_y 45°.

- `forward()` = direction from eye to target; `eye()` = `target − forward·distance`,
  where for yaw `y`, pitch `p` the eye offset direction is
  `(cos p · sin y, sin p, cos p · cos y)` (yaw 0, pitch 0: eye on +Z looking at −Z).
- `right()`, `up()`: camera basis (`right = forward × Y`, normalized; `up = right × forward`).
- `view()` = `look_at_rh(eye, target, +Y)`; `projection(aspect)`: perspective with
  near = `max(distance·1e-3, 1e-6)`, far = `distance·1e4`; orthographic spans
  `height·aspect × height` with near = `−distance·1e4`, far = `distance·1e4`.
  `view_proj(aspect)` = projection · view.
- `orbit(d_yaw, d_pitch)`: adds, then clamps pitch to `±(π/2 − 1e-4)` (no roll).
- `world_per_pixel(viewport_h)` at the target depth: perspective
  `2·distance·tan(fov/2)/h`, orthographic `height/h`.
- `pan(dx_px, dy_px, viewport_h)`: content follows the cursor:
  `target −= right·dx·wpp − up·dy·wpp` (screen y grows downward).
- `dolly(factor)`: `factor > 0`; perspective `distance *= factor` (min 1e-6),
  orthographic `height *= factor` (min 1e-6). Factors ≤ 0 are ignored.
- `frame(min, max)`: target = box centre, radius = half the diagonal (min 1e-6);
  perspective `distance = radius / sin(fov/2)`; orthographic `height = 2·radius`,
  `distance = max(distance, 2·radius)`. Yaw/pitch unchanged.
- `look_along(AxisView)` with `PosX, NegX, PosY, NegY, PosZ, NegZ`: eye placed on that
  side of the target (±Y uses the pitch clamp, yaw 0).
- Pixel space: origin top-left, x right, y down, continuous; pixel `(i, j)` covers
  `[i, i+1) × [j, j+1)`.
- `project(world, w, h) -> Option<DVec3>`: `(x_px, y_px, depth ∈ [0,1])`, `None` if
  the point is behind the camera (clip w ≤ 0) .
- `ray(x_px, y_px, w, h) -> Ray { origin, dir }` (`dir` normalized): perspective rays
  start at the eye; orthographic rays start on the near plane, all parallel to
  `forward()`. `project(ray.origin + ray.dir·t)` maps back to the same pixel.

## 2. Picking and rectangle selection (`picking`)

- `ray_triangle(ray, a, b, c) -> Option<f64>` (Möller–Trumbore, two-sided, `t > 1e-12`).
- `pick_poly(mesh, ray) -> Option<PolyHit { poly, t, point }>`: nearest hit over the
  triangulation of every polygon; ties → lower polygon index.
- `pick_point(mesh, cam, x, y, w, h, radius_px, visible_only) -> Option<u32>`: among
  points projecting within `radius_px` of the cursor, smallest screen distance, then
  smaller depth, then lower index. With `visible_only`, a point is skipped if a ray
  from the point toward the camera hits a polygon at `t > 1e-6·max(1, d)` (`d` =
  distance to the eye) and, in perspective, before reaching the eye; orthographic
  rays go toward `−forward()` unbounded.
- `pick_edge(...)`: same with the screen distance to the projected segment (edge ids
  from `PolyMesh::edges()`); visibility tested at the closest point on the edge.
- `rect_select(mesh, cam, rect, w, h, element, mode, visible_only) -> IndexSet` where
  `rect = [x0, y0, x1, y1]` in pixels (any corner order, inclusive), `element ∈
  {Points, Edges, Polys}`: points whose projection lies inside; edges/polygons by
  `Mode::All` (every vertex inside) or `Mode::Any`. **Parity-pending** defaults.

## 3. Snapping and constraint math (`snap`)

- `snap_scalar(v, inc)` = `round(v / inc) · inc` (`inc ≤ 0` → `v`).
- `snap_grid(p, spacing)`: per component.
- `ray_plane(ray, point, normal) -> Option<f64>`: `None` when
  `|dir·normal| < 1e-12` or `t < 0`.
- `line_param_closest_to_ray(origin, dir, ray) -> Option<f64>`: parameter `s` on the
  line `origin + dir·s` closest to the ray's line (Ericson §5.1.8); `None` when the
  two are parallel (`1 − (d̂₁·d̂₂)² < 1e-12`).
- `snap_to_point(mesh, cam, x, y, w, h, radius_px) -> Option<DVec3>`: world position
  of `pick_point(…, visible_only = true)`.

## 4. Grid (`grid`)

On the XZ plane (y = 0). `grid_lines(cam, viewport_h) -> Grid { spacing, lines }`:
- minor `spacing` = smallest power of ten ≥ `10 px · world_per_pixel(h)`
  (**parity-pending**); every 10th line is `Major`; the line through 0 is `AxisX`
  (along X, at z = 0) or `AxisZ` (along Z, at x = 0).
- Extent: ±50 minor cells around the target's XZ position snapped to a multiple of
  the major spacing (`10·spacing`).
- Order: lines parallel to X by ascending z, then lines parallel to Z by ascending x.
  `GridLine { a, b, kind }`.

## 5. Gizmo (`gizmo`) — translate and scale, world axes

`Gizmo { origin, mode: Translate | Scale, size_px }` (default `size_px` 100,
**parity-pending**). World length `L = size_px · wpp`, where `wpp` is the world size
of a pixel at the origin's depth (perspective: `2·d·tan(fov/2)/h`, `d` = origin's
distance along `forward`; orthographic: `height/h`).

Handles:
- `Axis(X|Y|Z)`: segment `origin → origin + L·axis`.
- `Plane(XY|YZ|XZ)`: square with corners at `0.2L` and `0.45L` along both plane axes.
- `Center`: disc of radius 8 px around the projected origin.

`hit_test(cam, x, y, w, h)`: `Center` if within 8 px; else a plane whose projected
quad contains the cursor (order XY, YZ, XZ); else the axis with the smallest screen
distance ≤ 6 px (ties: X, Y, Z order). Otherwise `None`. Handles seen edge-on are
not pickable: planes whose projected quad has area < 25 px², axes whose projected
length is < 6 px.

Drags: `Drag::begin(gizmo, handle, cam, x, y, w, h) -> Option<Drag>`, then
`drag.update(cam, x, y, w, h, snap: Option<f64>) -> DragDelta`:

| Mode / handle | Constraint | Result |
|---|---|---|
| Translate Axis | `s` = `line_param_closest_to_ray(origin, axis, ray)` | `Translate(axis · snap(s − s₀))` |
| Translate Plane | ray ∩ plane through origin, normal = missing axis | `Translate(snap_each(hit − hit₀))` |
| Translate Center | ray ∩ plane through origin, normal = `forward()` | `Translate(snap_each(hit − hit₀))` |
| Scale Axis | as translate axis; `begin` fails if `|s₀| < 1e-9` | `Scale` = 1 on other axes, `snap(s / s₀)` on the axis |
| Scale Plane | as translate plane; `begin` fails if `|hit₀ − origin| < 1e-9` | both plane axes `snap(|hit − origin| / |hit₀ − origin|)` |
| Scale Center | horizontal mouse motion | uniform `snap(1 + (x − x₀) / size_px)` |

`begin` also fails (`None`) if the start ray is parallel to the constraint. An
`update` whose ray is parallel returns the previous delta.

## 6. Renderer (`render`) — wgpu 30

- `Gpu::new_headless() -> Result<Gpu, GpuError>`: instance from env, any adapter
  (fallback allowed), default device limits.
- `MeshBuffers::new(&Gpu, &PolyMesh, &Selection)`: triangulated polygons (geometry §6)
  with per-corner flat normal, smooth normal (`corner_normals`, smoothing angle π,
  i.e. fully smooth), and selected flag; edge line list (selected flag per edge);
  point list, and one screen-space quad (two triangles) per point for sized points.
  `MeshBuffers::with_device_no_points` leaves the point buffers out.
- `Settings.point_size` (pixels, default 0): with `show_points`, 0 draws 1 px points (as
  before); above 0 every point is a depth-tested square of that side length centred on the
  projected point, with the same depth bias as edges, in the wire colour (selected points:
  the selection colour). Used by the component modes (EDITABLE.md §2.6).
- `Shading`: `Smooth`, `Flat`, `Wire`, `SmoothWire`, `FlatWire` and, since brief 143 (§8.3),
  `Quick`, `Constant`, `HiddenLine`, `BoundingBox`.
- `Scene` to draw: meshes with base colours, optional grid, optional gizmo, settings
  (background, shading, show points). Colours are linear RGB written unchanged to an
  `Rgba8Unorm` target (no sRGB conversion offscreen).
- Shaded fill: `base · (0.25 + 0.75 · |n·v|)`, `v = −forward()` (directional headlight,
  two-sided; this is the default `Lighting::Headlight`, the general rule is §8.1). Selected polygons use the selection colour as base. Wire overlay and
  wire mode draw edges in the wire colour (selected edges: selection colour); lines
  are 1 px, drawn with depth test `LessEqual` and a small depth bias toward the camera
  in overlay modes. Grid lines: depth-tested, no depth write. Gizmo: drawn last,
  without depth test; axes X red, Y green, Z blue.
- Default colours (**parity-pending**, our own palette): background (0.16, 0.17, 0.19),
  object (0.70, 0.70, 0.70), wire (0.05, 0.05, 0.06), selection (1.00, 0.55, 0.10),
  grid minor (0.22, 0.23, 0.25), major (0.30, 0.31, 0.34), axis X (0.55, 0.20, 0.20),
  axis Z (0.20, 0.25, 0.55).
- `render_to_rgba(&Gpu, w, h, &cam, &scene) -> Vec<u8>`: offscreen render, tightly
  packed RGBA rows, top row first.
- GPU tests run when an adapter exists; if none, they print a skip notice unless
  `FORGE_REQUIRE_GPU=1` is set, in which case they fail. CI installs a software
  Vulkan driver (lavapipe) and sets it.

## 7. Navigation (`nav`; brief 153)

Pure functions from a `Camera` to a new `Camera` (no GPU, no egui). Sources: own design from
docs/roadmap/areas/L-ui.md; GUI session G2 §2 (docs/parity/GUI-SESSIONS.md) is unanswered, so
**every direction, speed, margin and view orientation here is parity-pending G2**. Pixel
arguments are in the viewport's own pixels, y down.

| Function | Rule |
|---|---|
| `orbit_drag(cam, dx, dy)` | `orbit(−ORBIT_RAD_PER_PX·dx, +ORBIT_RAD_PER_PX·dy)`, `ORBIT_RAD_PER_PX = 0.01`. The pivot (`target`) and `distance` are unchanged; pitch clamps as in §1. |
| `pan_drag(cam, dx, dy, h)` | `Camera::pan`. The pivot moves inside the view plane (its offset has no `forward` component), by `wpp` per pixel. |
| `dolly_drag(cam, dx)` | `dolly(exp(−DOLLY_PER_PX·dx))`, `DOLLY_PER_PX = 0.01`: dragging right moves closer. Pivot unchanged. |
| `wheel_factor(s)` | `WHEEL_BASE^s`, `WHEEL_BASE = 0.999`, `s` in points, positive = zoom in. |
| `zoom_at(cam, cursor, size, f)` | Dolly by `f` and move the pivot so the point of the pivot plane (the plane through `target` facing the camera) that is under `cursor` stays under it. With `o = (cursor.x − w/2, h/2 − cursor.y)·wpp` taken before the dolly, `target += (right·o.x + up·o.y)·(1 − f)`. Exact for perspective and orthographic. `f ≤ 0` leaves the camera unchanged. Wheel zoom uses it; the Alt+secondary dolly drag zooms about the view centre (`dolly_drag`). |
| `frame_box(cam, min, max, aspect)` | World box → pivot at its centre, radius `r = FRAME_MARGIN · half diagonal`, `FRAME_MARGIN = 1.1`. Perspective: `distance = r / sin(min(fov_y, fov_x)/2)` with `tan(fov_x/2) = aspect · tan(fov_y/2)`, so the bounding sphere fits in both directions. Orthographic: `height = 2r · max(1, 1/aspect)`, `distance = max(distance, 2r)`. Yaw, pitch and projection kind are kept. A degenerate box (`r < 1e-9`, a single point) only recentres. |
| `view_camera(cam, view)` | `StandardView` = `Perspective, Front, Back, Left, Right, Top, Bottom`. The pivot is kept. The axis views switch to orthographic with `height` chosen so the apparent size at the pivot is unchanged (`2·distance·tan(fov_y/2)` when coming from perspective) and use `look_along`: Right = eye on world +X, Left = −X, Top = +Y, Bottom = −Y, Front = the world side the default editor camera sits on (−Z in a left-handed world, +Z in a right-handed one), Back = the opposite. `Perspective` returns to a perspective projection (`fov_y` 45°, `distance = height / (2·tan(fov_y/2))` when coming from orthographic) at the default orientation `DEFAULT_YAW = 0.68`, `DEFAULT_PITCH = 0.52`; if the camera is already perspective it is returned unchanged. |

The 4-view layout that this section deferred is §9 (brief 143).

Tests (crates/viewport/tests/nav.rs): orbit keeps pivot and distance and clamps pitch; pan
keeps distance and the offset is perpendicular to `forward`, content follows the cursor
(`project` of the old pivot moves by `(dx, dy)` pixels); dolly scales distance (and `height`)
and is the inverse for `1/f`; zoom-at keeps the pivot-plane point under the cursor fixed in
pixels (perspective and orthographic, off-centre cursors, both handednesses); frame fits all
eight corners of boxes inside the viewport for wide and tall aspects and both projections;
frame of a point recentres only; each standard view's eye side and forward vector.

## 8. Lights, shadows, anti-aliasing, display modes (brief 143)

Sources: published literature only: Lambert diffuse shading; L. Williams, "Casting curved shadows
on curved surfaces", SIGGRAPH 1978 (shadow maps); W. Reeves, D. Salesin, R. Cook, "Rendering
antialiased shadows with depth maps", SIGGRAPH 1987 (percentage-closer filtering); wgpu 30 and
WGSL public docs (multisampling, depth-texture comparison sampling). GUI session G8
(docs/parity/GUI-SESSIONS.md) is unanswered: **the mode names, the default rig, the number keys
and the arrangement of §9 are our own design, parity-pending G8.** Nothing here is a parity
claim; the render-parity work is briefs 137/145.

### 8.1 Lights (`lights`)

`Light { kind, direction, position, color, intensity, range, inner, outer, cast_shadow }`
(all `Copy`), `kind` = `Directional | Point | Spot`:

| Field | Meaning |
|---|---|
| `direction` | Directional and Spot: the unit direction the light travels (from the light into the scene). Normalised on use; a zero vector falls back to `-Y`. |
| `position` | Point and Spot: world position. Directional ignores it. |
| `color`, `intensity` | Linear RGB and a scalar; the light's contribution is `color · intensity`. |
| `range` | Point and Spot: 0 = no falloff; otherwise the factor is `(1 − min(d / range, 1))²`. |
| `inner`, `outer` | Spot: half-angles in radians (`inner ≤ outer`); the factor is `smoothstep(cos outer, cos inner, cos θ)`, `θ` = angle to the axis. |
| `cast_shadow` | Whether the light gets a shadow map (§8.2). |

Constructors: `Light::directional(direction, intensity)`, `Light::point(position, intensity)`,
`Light::spot(position, direction, inner, outer, intensity)`; `.with_shadow(bool)`,
`.with_color([f32; 3])`.

Lit colour of a fragment at world position `p` with normal `n` (two-sided, the viewport rule of
§6, so the normal is used as `|n·l|`):

`rgb = base · (AMBIENT + Σ_i  color_i · intensity_i · atten_i · |n·l_i| · lit_i)`,

`AMBIENT = 0.25`, `l_i` = unit vector towards light `i` (`−direction` for directional, `normalize(position − p)`
otherwise), `atten_i` = range and cone factors above (1 for directional), `lit_i` = 1, or the
shadow factor of §8.2. At most `MAX_LIGHTS = 8` lights are used; extra ones are ignored (the
first eight in order). The sum is clamped by the `Rgba8Unorm` target.

`Lighting` (in `Settings`, `Copy`):

- `Headlight` (default): one directional light `−forward()` (`direction = forward_world()`),
  intensity 0.75, no shadow. This is exactly the formula of §6, so the earlier images do not change.
- `Rig`: three world-anchored directional lights (our design): key (towards the light
  `(0.45, 0.85, 0.55)`, intensity 0.50, casts a shadow), fill (towards `(−0.70, 0.30, −0.40)`,
  0.20) and rim (towards `(0.00, −0.20, −1.00)`, 0.10). Vectors are normalised.
- `Scene(SceneLights)`: up to eight caller-supplied lights (`SceneLights { lights: [Light; 8], count }`,
  `SceneLights::new(&[Light])` caps at 8).

**How light objects plug in.** No light node type exists yet (rendering brief 137 adds the
light objects). The viewport takes plain `Light` values, so the editor needs one function
that turns the evaluated scene's light nodes into `Vec<Light>`: `forge_ui::panes::scene_lights`
(returns an empty list today). When it returns lights, `display_settings` passes them as
`Lighting::Scene` and they replace the headlight / rig; with none the editor's lighting mode
applies. A light node maps to `Light` by: omni → `Point`, spot → `Spot`, infinite → `Directional`
with the node's world-matrix −Z axis as `direction`; area lights are not representable and
may be approximated by `Point` at their centre. Nothing else in the renderer has to change.

### 8.2 Shadow maps

`Settings.shadows` (default false) enables them; `Settings.shadow_size` is the map side in texels
(default 1024, clamped to 64..=4096). Every light with `cast_shadow` that can have a shadow
camera gets one layer of a `Depth32Float` 2D-array texture: **cap 8 layers** (= `MAX_LIGHTS`),
layers are assigned in light order. `shadow_matrix(light, bounds) -> Option<DMat4>` (pure, f64):

- Bounds sphere: centre `c` = box centre, `r` = `max(half diagonal, 1e-6)`, of the union of the drawn
  meshes' bounds (`MeshBuffers::bounds`).
- Directional: orthographic, eye `c − d·1.05r`, looking at `c` (`d` = travel direction; up = +Y, or +X if `|d.y| > 0.99`),
  extents `±1.05r`, near 0, far `2.1r`. Every point of the bounds sphere maps inside NDC.
- Spot: perspective from `position` along `direction`, vertical field of view `min(2·outer + 0.1, 2.9)` rad, aspect 1,
  far `|c − position| + r`, near `max(|c − position| − r, 0.02·far, 1e-6)` (a tight near plane keeps the fixed NDC bias small relative to the depth steps).
- Point: **no shadow in this version** (`None`; needs a cube map, follow-up). `cast_shadow` is ignored for it.

The shadow pass renders the filled triangles of every mesh depth-only into each layer (slope-scaled
depth bias, constant 2, slope 2). It is skipped when the shading has no filled faces, when there
are no meshes, or when no light has a matrix. The shading pass compares each fragment's light-space
depth with the layer using a 3×3 percentage-closer filter (`textureSampleCompareLevel`, bias
`1.5e-3` in NDC depth): `lit_i` = fraction of the nine taps that pass. Outside the map (uv outside
[0,1] or depth beyond 1) the fragment is lit. A shadowed fragment keeps the ambient term.

### 8.3 Display modes (`Shading`, parity-pending G8)

| `Shading` | Draws |
|---|---|
| `Smooth` | filled faces, smooth normals, lit (Gouraud-like) |
| `Flat` | filled faces, face normals, lit |
| `Wire` | edges only |
| `SmoothWire`, `FlatWire` | the above fill plus edges |
| `Quick` | smooth normals with the **headlight only**: scene lights and shadows are ignored (the fast mode) |
| `Constant` | filled faces in the plain base colour, no lighting (selected faces in the selection colour) |
| `HiddenLine` | faces filled with the background colour (no light) and edges on top, so hidden edges are hidden |
| `BoundingBox` | the 12 edges of each mesh's axis-aligned bounding box in the wire colour, no faces |

Isoparms and skeleton display need spline parameter lines and joints, which the viewport does not
draw yet (briefs 112 and 157); they are not modes here. Shading, lights and shadows only affect
`Frame.meshes`; grid and gizmo are unchanged.
`MeshBuffers::bounds() -> Option<(DVec3, DVec3)>` is the box of the mesh's points (f32 precision).

### 8.4 Anti-aliasing

`Msaa { Off (default), X4 }` in `Settings`. Pipelines are built for one sample count:
`Renderer::new(device, format)` = 1 sample (as before), `Renderer::with_samples(device, format, 4)`.
With 4 samples the colour and depth attachments are multisampled and the colour is resolved into
the single-sample target (`RenderTargets` creates the three textures; `Renderer::render_to` records
the pass). `render_to_rgba` reads `Settings.msaa` and uses the matching renderer. Direct callers of
`Renderer::render` pass a resolve view when the colour target is multisampled. 4× is guaranteed
for `Rgba8Unorm` and `Depth32Float`. Lines and points get multisample coverage, no FXAA.

### 8.5 Tests

crates/viewport/tests/lights.rs (pure): light constructors normalise, `Lighting::Headlight` = the legacy
light (direction, 0.75, ambient 0.25), `Rig` lights unit-length and fixed in the world, `SceneLights::new`
caps at 8, `shadow_matrix` maps the bounds sphere inside NDC for directional and spot, `None` for point and
for `cast_shadow = false`. crates/viewport/tests/display.rs (software adapter; skips without one unless
`FORGE_REQUIRE_GPU=1`): per-mode behaviour (constant colour, hidden line hides the back edges, bounding box draws the
box, quick ignores scene lights), a spot and a point light lighting a plane (brighter near the axis),
a shadow under a box that disappears with `shadows = false`, 10 lights drawn like 8, MSAA edge pixels
(intermediate colours with `X4`, none with `Off`), and the baseline images below.

**Baseline images** (our own, not reference renders): `crates/viewport/tests/baselines/<name>.png`, one
per mode and per lighting setup, 160×120, produced on the software adapter (lavapipe). A test renders
the same scene and compares: on a software adapter the mean absolute channel error must be ≤ 1.0 and
at most 0.3 % of pixels may differ by more than 4 in any channel; on a hardware adapter ≤ 4.0 and 3 %
(rasteriser differences). `FORGE_UPDATE_BASELINES=1 cargo test -p forge-viewport --test display`
rewrites the files; a missing baseline fails the test with that hint. The tolerances are never
loosened to make a test pass; regenerate and review the images instead.

## 9. Panes and the 4-view layout (`panes`; brief 143)

Own design, parity-pending G8 (G8 §2 asks which views sit in which corners). Pure maths; the editor
turns it into egui (UI.md §6.2).

- `Pane` = `Perspective | Top | Front | Right`; text ids `perspective`, `top`, `front`, `right`.
- `Layout` = `Single | Four`.
- `visible(layout, active, maximized) -> Vec<Pane>`: `Single` → `[active]`; `Four` with `maximized = Some(p)` → `[p]`;
  `Four` otherwise → `[Perspective, Top, Right, Front]` (the quad order).
- `arrange(layout, active, maximized, width, height, gap) -> Vec<PaneRect>` with `PaneRect { pane, x, y, w, h }`
  in pixels, origin top-left. One visible pane gets the whole area. Four panes form a 2×2 grid
  separated by `gap` pixels: top-left Perspective, top-right Top, bottom-left Right, bottom-right Front.
  Column widths `w1 = floor((W − gap) / 2)`, `w2 = W − gap − w1`; row heights likewise, so the rects
  tile the area exactly (no overlap, gap strips between).
- `pane_at(&rects, x, y) -> Option<Pane>`: the rect containing the point (`x0 ≤ x < x0 + w`); gap strips and
  points outside give `None`.
- `pane_camera(pane, base) -> Camera`: the pane's default camera from the editor's default perspective camera
  `base`: `Perspective` = `base`; `Top`, `Front`, `Right` = `view_camera(base, Top/Front/Right)` (§7: orthographic, same pivot and apparent size).
- Each pane has its own camera and shading (state lives in the editor, UI.md §6.2). Lights and
  shadows are global; shading is per pane.

Tests (crates/viewport/tests/panes.rs): `visible` for every layout/maximise combination; `arrange` tiles exactly for odd and
even sizes and several gaps, quad corners, maximised and single give the full area; `pane_at` on
rect interiors, edges and gaps; default cameras: Top looks down −Y (forward ≈ −Y), Front/Right are orthographic with the
documented sides, Perspective is `base`, pivots equal.
