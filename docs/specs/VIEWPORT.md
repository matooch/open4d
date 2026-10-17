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

**Engine convention** (not a parity claim): right-handed, +Y up, as in glam. Mapping
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
  point list.
- `Shading`: `Smooth`, `Flat`, `Wire`, `SmoothWire`, `FlatWire`.
- `Scene` to draw: meshes with base colours, optional grid, optional gizmo, settings
  (background, shading, show points). Colours are linear RGB written unchanged to an
  `Rgba8Unorm` target (no sRGB conversion offscreen).
- Shaded fill: `base · (0.25 + 0.75 · |n·v|)`, `v = −forward()` (directional headlight,
  two-sided). Selected polygons use the selection colour as base. Wire overlay and
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
