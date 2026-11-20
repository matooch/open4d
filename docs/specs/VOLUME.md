# Volumes (`forge-volume`)

Sources: parity observations [docs/parity/volumes.md](../parity/volumes.md) (O32) and
[docs/parity/volumes-2.md](../parity/volumes-2.md) (O33), golden data `tests/golden/volumes/`;
area design [docs/roadmap/areas/F-volumes.md](../roadmap/areas/F-volumes.md); published
literature: K. Museth, "VDB: High-resolution sparse volumes with dynamic topology", ACM TOG
32(3) 2013 (sparse block grids, narrow-band level sets); A. Jacobson, L. Kavan,
O. Sorkine-Hornung, "Robust inside-outside segmentation using generalized winding numbers",
SIGGRAPH 2013; G. Barill et al., "Fast winding numbers for soups and clouds", SIGGRAPH 2018
(dipole approximation); C. Ericson, "Real-Time Collision Detection" 2004 (closest point on a
triangle); I. Quilez, "smooth minimum" (polynomial smooth-min); S. F. Gibson, "Constrained
elastic surface nets" 1998 and S. Frisken, R. Perry et al. on cell-based iso-surfacing (one vertex
per mixed cell, quads across crossed edges); H. Zhao, "A fast sweeping method for Eikonal
equations", Math. Comp. 2005.

Volumes are discretisation-sensitive, so parity is **geometric** with one exception: the
volume mesher (§7) turned out to reproduce the reference vertex positions and counts exactly for
the polygon sources, so those cases are also compared by counts (§7.4).

STATUS (kept in sync with docs/parity/STATUS.md): §1-§5 brief 080, see the table at the end.

## 1. Sparse grid (`SparseGrid<T>`)

- Index space: integer voxel indices `(i, j, k)`. World position of a voxel sample is
  `origin + (i, j, k) * voxel_size` (a uniform scale and a translation; the reference's
  "Override Grid Matrix" is not observed and not supported). Samples sit at multiples of the
  voxel size from the origin; the default origin is the world origin (**O32**: every observed
  mesh vertex lies at `10 n + 5` or on a source face, i.e. samples at multiples of 10 at voxel
  size 10).
- Storage: a hash map from the block key `floor(index / 8)` to a dense block of 8×8×8 values
  and an active bit mask (512 bits). A voxel that was never stored reads as the grid's
  `background` value and is inactive. Setting a value activates it; `set_inactive` stores a value
  without activating it (used for the filled interior, §2.4).
- Iteration over active voxels is in ascending `(i, j, k)` order (deterministic); `get`, `set`,
  `is_active`, `active_count`, `bounds` (index box of the active voxels) and `trilinear`
  sampling (inactive samples read as the background) are the API.
- Values are `f32` for SDF and fog grids (the VDB float grid of brief 086).

## 2. Mesh to signed distance field

`mesh_to_sdf(mesh, &SdfParams)` takes a world-space polygon mesh. Polygons are triangulated as
a fan from corner 0 (flat quads are unaffected).

1. **Band.** `SdfParams { voxel_size, origin, exterior_range, interior_range }`. The exterior band
   is `exterior_range * voxel_size` wide, the interior band `interior_range * voxel_size`
   (O32 defaults 2 and 2).
2. **Unsigned distance.** For each triangle every voxel in its bounding box grown by the larger
   band gets the exact distance from the voxel sample to the triangle (closest point on the
   triangle); each voxel keeps the minimum over triangles.
3. **Sign.** The generalized winding number of the whole mesh at the sample (Jacobson 2013)
   decides: `w > 0.5` is inside. Sums use the exact solid angle of van Oosterom and Strackee for
   near triangles and a dipole approximation (Barill 2018, accuracy parameter 2) for far
   clusters of a bounding volume hierarchy. A sample whose distance is below `1e-9 * voxel_size` is
   treated as exactly on the surface: its value is `+0.0` (outside, **O32 cube-defaults**: the
   samples on the cube faces sit on the outside of the iso-surface, §7.1).
4. **Values.** Outside: `+d`, kept active when `d <= exterior band`. Inside: `-d`, kept active
   when `d <= interior band`. Voxels beyond their band are not stored; the grid background is
   `+exterior band`.
5. **Interior fill** (`signed_flood_fill`). Between two negative active voxels of the same x
   row with no active voxel between them, every unstored voxel is set (inactive) to
   `-interior band`. This gives CSG (§4) the right value deep inside a solid. Rows that
   leave the solid between two bands (a hollow, a thin shell) get no fill because an active
   voxel of the shell lies between. **Spec decision, not observed.**

## 3. Spline and points to signed distance

- `spline_to_sdf(polyline, closed, radius, params)`: value = distance to the nearest polyline
  segment minus `radius` (a chain of capsules), same band and fill rules (**own design**; the
  tube radius the reference uses for splines is derived in §6.3).
- `points_to_sdf(points, radii, params)`: a union of spheres, `min_i(|p - c_i| - r_i)`;
  `PointUnion::Smooth(k)` switches the union to the polynomial smooth-min of §4. Only voxels
  within `r + band` of a point are visited. Values keep the band rule of §2.4: a sample deeper
  inside than the interior band is not stored and reads `-interior band` after the fill.

## 4. CSG

On SDF grids sampled on the same lattice (same voxel size and origin; otherwise
`VolumeError::GridMismatch`):

| op | value |
|---|---|
| union | `min(a, b)` |
| subtract (`a - b`) | `max(a, -b)` |
| intersect | `max(a, b)` |
| negate | `-a` |
| smooth union with width `k > 0` | `h = max(k - |a - b|, 0) / k`, `min(a, b) - h * h * k / 4` |

A voxel is active in the result when it is active in either input (backgrounds participate with
their values, so the unstored interior of §2.5 combines correctly). The result background is the
operation applied to the backgrounds.

## 5. Harness: `compare_volume_mesh(expected, actual, voxel)`

`forge_parity_harness::compare_volume_mesh` returns a `VolumeMeshReport`; `passed()` requires:

- **Hausdorff**: both directions, from the vertices and the face centroids of each mesh to the
  triangulated other mesh, at most `voxel / 2`;
- **area** and **volume** (closed meshes) each within 2 %;
- the same **Euler characteristic** `V - E + F` of the welded meshes (`WELD_TOLERANCE`).

## 6. Volume Builder (`forge.volume_builder`)

Category `Generator`, label "Volume Builder". It produces a volume, not a polygon mesh: its own
geometry evaluation is `None` and the mesher (§7) or another consumer reads the volume with
`build_volume(scene, builder, ctx)`.

### 6.1 Parameters (ids and defaults from O32; the defaults are observed starting values)

| id | label | kind | default | range / options |
|---|---|---|---|---|
| `voxel_size` | Voxel Size | Float, distance | 10 | 0.001 .. 1e20, step 0.1 |
| `interior_voxel_range` | Interior Voxel Range | Int | 2 | 1 .. 2147483647 |
| `exterior_voxel_range` | Exterior Voxel Range | Int | 2 | 1 .. 2147483647 |
| `spline_voxel_range` | Spline Voxel Range | Int | 2 | 1 .. 2147483647 |
| `particle_voxel_range` | Particle Voxel Range | Int | 2 | 1 .. 2147483647 |
| `volume_type` | Volume Type | Enum `sdf, fog, vector` | `sdf` | `vector` is `Unsupported` |
| `auto_update_settings` | Auto Update Settings | Bool | on | stored only |
| `override_grid_matrix` | Override Grid Matrix | Bool | off | on is `Unsupported` (not observed) |
| `objects` | Objects | LinkRows, blends `union, subtract, intersect` | empty | §6.2 |

### 6.2 The object list

- The list is the builder's **children in tree order** (O32: "the builder picks up its direct
  children"; no separate add call exists). A child without geometry of its own (a Null) is
  traversed: its children are entries with their full world transforms (**O32 hierarchy**, both
  cubes under the Null contributed). There is no "use hierarchy" switch (the O32 notes).
- The `objects` rows are **overrides** keyed by child: a row names the child's blend (`union`
  default) and `enabled`. Children without a row are enabled unions. The per-entry operation is
  not scriptable in the reference (O32 subtract / intersect: not observed), so the three blends
  are **parity-pending, own design**: `union`, `subtract` (`acc - entry`), `intersect`.
- A disabled node (`object.enabled` off) or a disabled row is skipped.
- Entries are applied top to bottom; the **first** entry initialises the accumulator whatever its
  blend (own design).
- Entry kinds:
  - **polygon objects** (anything `forge_generators::evaluate` turns into a mesh, clones
    included) go through `mesh_to_sdf` (§2) in builder space with the interior and exterior
    ranges;
  - **splines** (anything `splines::polyline_mesh` accepts) go through `spline_to_sdf` (§3) with the
    spline radius of §6.3 and `spline_voxel_range`;
  - **volume builders** as children: their volume, resampled trilinearly onto this lattice when the
    lattice differs (**own design**);
  - filter layers (§8) and field layers (§9) belong to briefs 082 and 085.
- Entry meshes are placed with `world(builder)^-1 * world(child)`, so the grid is in the builder's
  local space; the mesher puts the mesh into its own space with the builder's local transform.

### 6.3 Spline radius

A spline entry becomes a tube of radius `0.735 * spline_voxel_range * voxel_size`
(`SPLINE_RADIUS_PER_RANGE`; §3 capsule chain over the spline's sampled polyline). Derived from
**O33 spline-source and spline-radius**: sweeping the radius against the golden meshes, the
polygon area matches at 14.71 (spline-source) and 14.68 (spline-radius) at voxel size 10, range 2.
The factor is the only fit and only the range 2 is observed, so it is **parity-pending**;
the helix itself (O33: start and end radius 200 or 20, 4 pi, height 200) and the golden both run
along -z from 0 to -200 (the golden bbox z -213.9 .. 12.7), which our helix matches.
The scripted case form stores percent parameters as fractions (`radial_bias = 0.5` is 50 %).

### 6.4 Fog

`volume_type = fog` builds the distance field as above, then maps it to a density with
`fog = clamp((-sdf - 0.7 voxel) / (4 voxel), 0, 1)` (`sdf_to_fog`): 0 outside, on the surface and
up to 0.7 voxel inside, 1 from 4.7 voxels inside; samples at or beyond the interior band (`-interior band`, including
the filled interior of §2.5) are 1 so a deep interior is full density; background 0. The fog is meshed with inside
above the threshold (§7.3). **O33 fog-input** (one case): the reference mesh has 1664 points
and its surface lies about 6 to 8 units inside the distance-field surface at voxel size 10
(bbox x +-93.3 against +-98.0); sweeping an offset and a ramp, `0.7` and `4` give the
geometric match (Hausdorff 2.5 below voxel / 2, area +1.3 %) with 1568 points. The formula is
a fit, **parity-pending**: the reference's own density function is not derived (an
OpenVDB-style `sdf-to-fog` with the surface at density 0 gives 1832 points and a Hausdorff of
10). `vector` volumes are `Unsupported`.

## 7. Mesher (`forge.volume_mesher`)

Category `Generator`, label "Volume Mesher"; its first builder child is built (§6) and meshed in
the mesher's own space (the builder's local transform applied). No builder child: no mesh.

### 7.1 Parameters (O32)

| id | label | kind | default | notes |
|---|---|---|---|---|
| `surface_threshold` | Surface Threshold | Float | 0.01 | -1e20 .. 1e20, step 0.01; §7.2 |
| `voxel_range_threshold` | Voxel Range Threshold | Float | 0.5 | 0 .. 1, stored, no observed effect |
| `use_absolute_value_iso` | Use Absolute Value (ISO) | Bool | off | stored (no observed effect) |
| `adaptive` | Adaptive | Float | 0 | 0 .. 1; brief 083 |
| `create_curvature_map` | Create Curvature Map | Bool | off | on is `Unsupported` |
| `curvature_direction` | Curvature Direction | Enum `both, convex, concave` | `both` | stored |

### 7.2 Surface extraction (adaptive 0)

Derived from **O32 cube-defaults** and confirmed on every polygon case (all quads, point and
face counts equal to the reference):

1. A lattice sample is **inside** when its value is below the iso value (distance fields) and
   **outside** otherwise, so a sample exactly on the surface (value `0`) is outside. For the
   default cube (voxel 10, faces at +-100) the outermost samples are at +-100 (outside) and the
   inside starts at +-90.
2. Every lattice cell (the 8 samples around it) with both sides gets **one vertex**: the mean of
   the points where the iso value crosses its 12 edges (linear interpolation of the sample
   values). For the cube this gives the observed -93.333 corner vertices (three crossings at
   -100, -90, -90) and -95 / -85 edge and face vertices; the face vertex `(-100, -85, -85)` is
   the mean of four crossings at exactly -100.
3. Every lattice edge with an inside and an outside end gets a **quad** from the four cells that
   share it, wound counter-clockwise seen from outside.
4. Mixed cells with more than two sign regions still get one vertex (no case seen); no
   triangles come out at adaptive 0 (O32: all quads).

Result, 2168 points / 2166 faces for the default cube, every vertex position equal to 1e-3 to
the reference (sorted comparison); the same counts for sphere (1832/1830), sphere at voxel 5
(7328/7326), cube at voxel 5 and 20 (9128/9126, 488/486), union-two (2952/2950), hierarchy
(3384/3382) and the platonic of `torus-defaults` (1400/1398).

The surface threshold: **O32 mesher-threshold** (0.5 at voxel 10) is byte-identical to 0.01, so
for distance fields a threshold within `0.05 voxel` of zero (inclusive) means iso 0; larger
magnitudes shift the iso value unchanged. Parity-pending (only one probe value).

### 7.3 Fog

Fog volumes are meshed with inside above the threshold (`HighInside`); the threshold applies as
given (0.01 default, **O33 fog-input**).

### 7.4 Parity

Cases are compared with `compare_volume_mesh` at the voxel size (§5) and, for the polygon
sources, also by point and face count (strict). The reference quad numbering and start corners
are not reproduced; the fan triangulation of non-planar quads at the cube edges makes the
two-sided Hausdorff 0.43 (cube, voxel 10), well below voxel / 2.
