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

## 6. Builder

*(brief 081, filled in with that brief.)*

## 7. Mesher

*(brief 083, filled in with that brief.)*
